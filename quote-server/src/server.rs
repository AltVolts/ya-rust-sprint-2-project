use crate::client_registry::{ClientInfo, ClientRegistry};
use anyhow::{Result as AnyhowResult, anyhow};
use log::{error, info, warn};
use quote_core::ticker_list::get_tickers_from_txt;
use quote_core::{TickerPrices, TickerPricesExt};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::str::SplitWhitespace;
use std::sync::{Arc, Mutex};

pub(crate) fn handle_client(
    stream: TcpStream,
    client_registry: Arc<Mutex<ClientRegistry>>,
    socket: Arc<UdpSocket>,
) {
    let writer = match stream.try_clone() {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to clone stream: {}", e);
            return;
        }
    };
    let mut writer = BufWriter::new(writer);
    let mut reader = BufReader::new(stream);

    if let Err(e) = writer.write_all(b"OK: You connected to quote-server server!\n") {
        error!("Failed to send welcome message: {}", e);
        return;
    }
    if let Err(e) = writer.flush() {
        error!("Failed to flush welcome message: {}", e);
        return;
    }

    let mut line = String::new();
    let n = match reader.read_line(&mut line) {
        Ok(0) => {
            info!("Client disconnected");
            return;
        }
        Ok(n) => n,
        Err(e) => {
            error!("Error reading from client: {}", e);
            return;
        }
    };
    let input = line.trim();
    if input.is_empty() {
        info!("Client disconnected");
        send_error(&mut writer, "command is empty");
        return;
    }

    let (command, parts) = match parse_command(input) {
        Ok(cmd) => cmd,
        Err(e) => {
            error!("Failed to parse client command: {}", e);
            send_error(&mut writer, &e.to_string());
            return;
        }
    };
    _ = match command {
        "STREAM" => {}
        "EXIT" => {
            let message = "OK: goodbye\n";
            if let Err(e) = writer.write_all(message.as_bytes()) {
                error!("Failed to send error: {}", e);
            }
            if let Err(e) = writer.flush() {
                error!("Failed to flush error: {}", e);
            }
            return;
        }
        _ => {
            let message = format!("ERROR: wrong command {}\n", command);
            send_error(&mut writer, message.as_str());
            return;
        }
    };

    let (udp_addr, tickers_set) = match handle_stream_cmd(parts) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to parse client command: {}", e);
            send_error(&mut writer, &e.to_string());
            return;
        }
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let client = ClientInfo::new(udp_addr, tx);
    let mut registry = match client_registry.lock() {
        Ok(guard) => guard,
        Err(e) => {
            error!("Failed to lock client registry: {}", e);
            send_error(
                &mut writer,
                format!("internal server error - {e}\n").as_str(),
            );
            return;
        }
    };

    if let Err(e) = registry.add_client(client) {
        error!("Failed to add client to registry: {}", e);
        send_error(
            &mut writer,
            format!("internal server error - {e}\n").as_str(),
        );
        return;
    }

    let mut failed_count = 0;
    loop {
        if failed_count > 10 {
            error!("To much handle client errors");
            client_registry
                .lock()
                .unwrap()
                .remove_client(udp_addr)
                .unwrap();
            send_error(&mut writer, "ERROR: To much handle client errors");
            return;
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
            return;
        }

        let encoded = match filtered_prices.serialize() {
            Ok(encoded) => encoded,
            Err(e) => {
                failed_count += 1;
                error!("Failed to serialize ticker prices: {}", e);
                continue;
            }
        };
        match socket.send_to(&encoded, udp_addr) {
            Ok(_) => n,
            Err(e) => {
                failed_count += 1;
                error!("Failed to send ticker prices: {}", e);
                continue;
            }
        };
    }
}

fn filter_prices(prices: TickerPrices, tickers_set: &HashSet<String>) -> TickerPrices {
    prices
        .into_iter()
        .filter(|(ticker, _)| tickers_set.contains(ticker))
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
    if port == 0 || port > 65535 {
        return Err(anyhow!("port out of range (1-65535)"));
    }

    let udp_addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|_| anyhow!("host is not a valid IP address"))?;

    let tickers: Vec<&str> = tickers_str.split(',').collect();
    if tickers.is_empty() {
        return Err(anyhow!("client ticker list is empty"));
    }

    let tickers_list = get_tickers_from_txt(crate::DEFAULT_TICKERS_PATH).map_err(|e| {
        anyhow!(
            "failed to read tickers from file {}: {}",
            crate::DEFAULT_TICKERS_PATH,
            e
        )
    })?;

    for ticker in &tickers {
        if ticker.is_empty() {
            return Err(anyhow!("client ticker cannot be empty"));
        }
        if !tickers_list.contains(*ticker) {
            return Err(anyhow!(
                "file's tickers list doesn't contain client ticker {}",
                ticker
            ));
        }
    }

    Ok((udp_addr, tickers_list))
}
