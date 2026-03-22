use crate::client_registry::{ClientInfo, ClientRegistry};
use anyhow::{Context, Result as AnyhowResult, anyhow};
use log::{error, info, warn};
use quote_core::{StockQuote, TickerPrices, serialize_quotes};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::str::SplitWhitespace;
use std::sync::{Arc, Mutex};

pub(crate) fn handle_client(
    stream: TcpStream,
    client_registry: Arc<Mutex<ClientRegistry>>,
    socket: Arc<UdpSocket>,
) -> AnyhowResult<()> {
    let writer = stream
        .try_clone()
        .with_context(|| "Failed to clone stream")?;
    let mut writer = BufWriter::new(writer);
    let mut reader = BufReader::new(stream);

    writer
        .write_all(b"OK: You connected to quote-server server!\n")
        .with_context(|| "Failed to send welcome message")?;
    writer
        .flush()
        .with_context(|| "Failed to flush welcome message")?;

    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => {
            info!("Client disconnected");
            return Ok(());
        }
        Ok(_) => {}
        Err(e) => {
            error!("Error reading from client: {}", e);
            return Err(anyhow!("Error reading from client: {}", e));
        }
    }

    let input = line.trim();
    if input.is_empty() {
        info!("Client disconnected");
        send_error(&mut writer, "command is empty");
        return Ok(());
    }

    let (command, parts) = parse_command(input).map_err(|e| {
        error!("Failed to parse client command: {}", e);
        send_error(&mut writer, &e.to_string());
        anyhow!("Failed to parse client command: {}", e)
    })?;

    match command {
        "STREAM" => {}
        "EXIT" => {
            let message = "OK: goodbye\n";
            writer
                .write_all(message.as_bytes())
                .with_context(|| "Failed to send goodbye")?;
            writer.flush().with_context(|| "Failed to flush goodbye")?;
            return Ok(());
        }
        _ => {
            let message = format!("ERROR: wrong command {}\n", command);
            send_error(&mut writer, &message);
            return Err(anyhow!("wrong command {}", command));
        }
    }

    let (udp_addr, tickers_set) = handle_stream_cmd(parts).map_err(|e| {
        error!("Failed to parse client command: {}", e);
        send_error(&mut writer, &e.to_string());
        anyhow!("Failed to parse client command: {}", e)
    })?;

    let (tx, rx) = std::sync::mpsc::channel();
    let client = ClientInfo::new(udp_addr, tx);
    {
        let mut registry = client_registry.lock().map_err(|e| {
            error!("Failed to lock client registry: {}", e);
            send_error(&mut writer, &format!("internal server error - {}\n", e));
            anyhow!("Failed to lock client registry: {}", e)
        })?;

        registry.add_client(client).map_err(|e| {
            error!("Failed to add client to registry: {}", e);
            send_error(&mut writer, &format!("internal server error - {}\n", e));
            anyhow!("Failed to add client to registry: {}", e)
        })?;
    }

    let mut failed_count = 0;
    loop {
        if failed_count > 10 {
            error!("Too many handle client errors");
            if let Ok(mut registry) = client_registry.lock() {
                let _ = registry.remove_client(udp_addr);
            }
            send_error(&mut writer, "ERROR: Too many handle client errors");
            return Err(anyhow!("Too many handle client errors"));
        }

        let ticker_prices = match rx.recv() {
            Ok(ticker_prices) => ticker_prices,
            Err(e) => {
                failed_count += 1;
                error!("Failed to receive ticker_prices: {}", e);
                continue;
            }
        };

        let filtered_prices = filter_prices(ticker_prices, &tickers_set);
        if filtered_prices.is_empty() {
            error!("No tickers after client filtration");
            send_error(&mut writer, "ERROR: no tickers were found");
            return Err(anyhow!("No tickers after client filtration"));
        }

        let encoded = match serialize_quotes(filtered_prices) {
            Ok(encoded) => encoded,
            Err(e) => {
                failed_count += 1;
                error!("Failed to serialize ticker prices: {}", e);
                continue;
            }
        };
        match socket.send_to(&encoded, udp_addr) {
            Ok(_) => {}
            Err(e) => {
                failed_count += 1;
                error!("Failed to send ticker prices: {}", e);
                continue;
            }
        }
    }
}

fn filter_prices(prices: TickerPrices, tickers_set: &HashSet<String>) -> Vec<StockQuote> {
    prices
        .into_iter()
        .filter(|(ticker, _)| tickers_set.contains(ticker))
        .map(|(_, stock)| stock)
        .collect()
}

fn send_error(writer: &mut BufWriter<TcpStream>, msg: &str) {
    let err_msg = format!("ERROR: {}\n", msg);
    if let Err(e) = writer.write_all(err_msg.as_bytes()) {
        error!("Failed to send error: {}", e);
    }
    if let Err(e) = writer.flush() {
        error!("Failed to flush error: {}", e);
    }
}

fn parse_command(input: &str) -> AnyhowResult<(&str, SplitWhitespace)> {
    let mut parts = input.split_whitespace();
    let command = parts.next().ok_or_else(|| anyhow!("command is empty"))?;
    Ok((command, parts))
}

fn handle_stream_cmd(mut parts: SplitWhitespace) -> AnyhowResult<(SocketAddr, HashSet<String>)> {
    let url_str = parts.next().ok_or_else(|| anyhow!("missing UDP URL"))?;
    let tickers_str = parts.next().ok_or_else(|| anyhow!("missing ticker list"))?;

    if parts.next().is_some() {
        return Err(anyhow!("too many arguments"));
    }

    let url = url::Url::parse(url_str).map_err(|_| anyhow!("invalid URL format"))?;
    if url.scheme() != "udp" {
        return Err(anyhow!("URL scheme must be 'udp'"));
    }
    let host = url
        .host()
        .ok_or_else(|| anyhow!("URL missing host"))?
        .to_string();
    let port = url.port().ok_or_else(|| anyhow!("URL missing port"))?;

    let udp_addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|_| anyhow!("host is not a valid IP address"))?;

    let tickers: Vec<&str> = tickers_str.split(',').collect();
    if tickers.is_empty() {
        return Err(anyhow!("client ticker list is empty"));
    }
    let tickers_set: HashSet<String> = tickers.into_iter().map(|s| s.to_string()).collect();

    Ok((udp_addr, tickers_set))
}
