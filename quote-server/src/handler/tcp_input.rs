use anyhow::{Context, Result as AnyhowResult, anyhow};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpStream};
use std::str::SplitWhitespace;

pub(super) fn handle_tcp_input(stream: TcpStream) -> AnyhowResult<(SocketAddr, HashSet<String>)> {
    let writer = stream
        .try_clone()
        .with_context(|| "Failed to clone stream")?;
    let mut writer = BufWriter::new(writer);
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .with_context(|| "Failed to clone stream")?,
    );

    writer
        .write_all(b"OK: You connected to quote-server server!\n")
        .with_context(|| "Failed to send welcome message")?;
    writer
        .flush()
        .with_context(|| "Failed to flush welcome message")?;

    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => return Err(anyhow!("Client disconnected")),
        Ok(_) => {}
        Err(e) => {
            return Err(anyhow!("Error reading from client: {}", e));
        }
    }

    let input = line.trim();
    parse_tcp_command(input)
}

fn parse_tcp_command(input: &str) -> AnyhowResult<(SocketAddr, HashSet<String>)> {
    let (command, parts) =
        parse_command(input).map_err(|e| anyhow!("Failed to parse client command: {}", e))?;

    match command {
        "STREAM" => {}
        _ => {
            return Err(anyhow!("wrong command {}", command));
        }
    }

    let (udp_addr, tickers_set) =
        parse_command_parts(parts).map_err(|e| anyhow!("Failed to parse client command: {}", e))?;
    Ok((udp_addr, tickers_set))
}

fn parse_command(input: &str) -> AnyhowResult<(&str, SplitWhitespace)> {
    let mut parts = input.split_whitespace();
    let command = parts.next().ok_or_else(|| anyhow!("command is empty"))?;
    Ok((command, parts))
}

fn parse_command_parts(mut parts: SplitWhitespace) -> AnyhowResult<(SocketAddr, HashSet<String>)> {
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
