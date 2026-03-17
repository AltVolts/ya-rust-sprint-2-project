use std::collections::HashSet;
use anyhow::Result;
use log::{error, warn};
use quote_core::QuoteGenerator;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use quote_core::ticker_list::get_tickers_from_txt;

const DEFAULT_TICKERS_PATH: &str = "../../tickers.txt";

pub(crate) fn handle_client(stream: TcpStream, quote_generator: Arc<Mutex<QuoteGenerator>>) {
    let mut writer = stream.try_clone().expect("Failed to clone stream");
    let mut reader = BufReader::new(stream);

    let _ = writer.write_all(b"You connected to quote-server server!");
    let _ = writer.flush();

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                return;
            }
            Ok(_) => {
                let input = line.trim();
                if input.is_empty() {
                    let _ = writer.flush();
                    continue;
                }
                let response = process_command(input, &quote_generator);
                if let Err(e) = writer.write_all(response.as_bytes()) {
                    log::error!("Failed to send response: {}", e);
                    break;
                }
                if let Err(e) = writer.flush() {
                    log::error!("Failed to flush: {}", e);
                    break;
                }
            }
            Err(e) => {
                error!("Error reading from client: {}", e);
                break;
            }
        }
    }
}

fn process_command(input: &str, quote_generator: &Arc<Mutex<QuoteGenerator>>) -> String {
    let mut parts = input.split_whitespace();
    let command = match parts.next() {
        Some(cmd) => cmd,
        None => return "ERROR: empty command\n".to_string(),
    };

    match command {
        "STREAM" => handle_stream_cmd(parts, quote_generator),
        "EXIT" => "OK: goodbye\n".to_string(),
        _ => format!("ERROR: unknown command '{}'\n", command),
    }
}

fn handle_stream_cmd(
    mut parts: std::str::SplitWhitespace,
    quote_generator: &Arc<Mutex<QuoteGenerator>>,
) -> String {
    // Должно быть ровно две части: URL и списк тикеров
    let url_str = match parts.next() {
        Some(u) => u,
        None => return "ERROR: missing UDP URL\n".to_string(),
    };
    let tickers_str = match parts.next() {
        Some(t) => t,
        None => return "ERROR: missing ticker list\n".to_string(),
    };
    // Обработка лишних ключей в команде
    if parts.next().is_some() {
        return "ERROR: too many arguments\n".to_string();
    }

    let url = match url::Url::parse(url_str) {
        Ok(u) => u,
        Err(_) => return "ERROR: invalid URL format\n".to_string(),
    };
    if url.scheme() != "udp" {
        return "ERROR: URL scheme must be 'udp'\n".to_string();
    }
    let host = match url.host() {
        Some(h) => h.to_string(),
        None => return "ERROR: URL missing host\n".to_string(),
    };
    let port = match url.port() {
        Some(p) => p,
        None => return "ERROR: URL missing port\n".to_string(),
    };
    // Проверим, что порт в допустимом диапазоне (1–65535)
    if port == 0 || port > 65535 {
        return "ERROR: port out of range (1-65535)\n".to_string();
    }

    // Преобразуем host и port в SocketAddr для отправки UDP.
    let udp_addr: std::net::SocketAddr = match format!("{}:{}", host, port).parse() {
        Ok(addr) => addr,
        Err(_) => {
            return "ERROR: host is not a valid IP address\n".to_string();
        }
    };

    // 2. Разбор списка тикеров
    let tickers: Vec<&str> = tickers_str.split(',').collect();
    if tickers.is_empty() {
        return "ERROR: client ticker list is empty\n".to_string();
    }

    let tickers_res = get_tickers_from_txt(crate::DEFAULT_TICKERS_PATH);
    let Some(tickers_list) = tickers_res.ok() else {
        return format!("ERROR: tickers list from the file {} is empty\n", DEFAULT_TICKERS_PATH);
    };

    for ticker in &tickers {
        if ticker.is_empty() {
            return "ERROR: client ticker cannot be empty\n".to_string();
        }
        if !tickers_list.contains(*ticker) {
            return format!("ERROR: file's tickers list doesnt contain client ticker {}\n", ticker);
        }
    }

    // 3. Регистрация подписки в генераторе котировок
    // Предположим, у QuoteGenerator есть метод `add_subscriber(udp_addr, tickers)`
    {
        let mut generator = quote_generator.lock().unwrap();
        if let Err(e) =
            generator.add_subscriber(udp_addr, tickers.iter().map(|&s| s.to_string()).collect())
        {
            return format!("ERROR: failed to register subscription: {}\n", e);
        }
    }

    format!("OK: streaming {} to {}\n", tickers.join(","), udp_addr)
}
