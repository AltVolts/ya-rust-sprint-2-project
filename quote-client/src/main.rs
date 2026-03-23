use crate::cli::Cli;
use crate::file::read_tickers_file;
use crate::receiver::QuoteReceiver;
use crate::response::ServerTcpResponse::ErrorResponse;
use crate::response::is_response_ok;
use anyhow::{Context, anyhow};
use env_logger::{Builder, Env};
use log::{error, info};
use quote_core::StockQuote;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

mod cli;
mod file;
mod keep_alive;
mod receiver;
mod response;

const POLL_INTERVAL_MS: u64 = 2000;
const TCP_RESPONSE_MAX_TIMEOUT: u64 = 6000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Cli::get_args();

    // Configure shutdown flag and ctrlc handler for graceful shutdown
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    ctrlc::set_handler(move || {
        shutdown_clone.store(true, Ordering::Relaxed);
        info!("Получен сигнал завершения, начинаем остановку...");
    })?;

    // Code block of send STREAM command to server and receiving response. If response not 'OK' client stops working
    {
        let tickers_str = read_tickers_file(args.tickers_file)?;
        let command = format!("STREAM udp://{} {}\n", args.udp_port, tickers_str);

        let mut stream = TcpStream::connect(args.server_addr)?;
        stream.set_read_timeout(Some(Duration::from_millis(TCP_RESPONSE_MAX_TIMEOUT)))?;
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .with_context(|| "Failed to clone stream")?,
        );

        stream.write_all(command.as_bytes())?;
        stream.flush()?;

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(Box::from(anyhow!("Server disconnected"))),
            Ok(_) => {}
            Err(e) => {
                return Err(Box::from(anyhow!("Error reading from server: {}", e)));
            }
        }
        if let ErrorResponse(error) = is_response_ok(line) {
            return Err(Box::from(anyhow!("Error reading from server: {}", error)));
        }
        if let Err(e) = stream.shutdown(Shutdown::Both) {
            error!("Ошибка при закрытии TCP-соединения: {}", e);
        }
    }

    /* Set up quote receiving thread keep-alive thread
    After initializing quote receiver get from it a configured udp socket for keep-alive thread.as
    It also requires server address which is required from quote receiver via server_addr_rx channel rx.
    */
    let quotes_receiver = QuoteReceiver::new(args.udp_port, shutdown.clone())?;
    let udp_socket = quotes_receiver.get_socket_clone();
    let (receive_handle, quotes_rx, server_addr_rx) =
        quotes_receiver.start_with_channel_and_server_addr();

    let server_addr = match server_addr_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(addr) => addr,
        Err(e) => {
            error!("Не удалось получить адрес сервера: {}", e);
            return Err(e.into());
        }
    };
    let _ping_handle = keep_alive::start_ping_sender(udp_socket, server_addr, shutdown.clone());

    /* Main loop
    At the begging of each iteration check shutdown flag.
    Receiving data via channel from receiver configured with timeout preventing blocking the loop
    to check shutdown flag periodically.
    */
    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        match quotes_rx.recv_timeout(Duration::from_millis(POLL_INTERVAL_MS)) {
            Ok((mut stocks, addr)) => {
                info!(
                    "[ОСНОВНОЙ ПОТОК] Получено котировок от сервера '{}' - {}",
                    addr,
                    stocks.len()
                );
                stocks.sort_by(|a, b| a.ticker.cmp(&b.ticker));
                info!("{}", StockQuote::header());
                info!("{}", StockQuote::separator());
                for stock in stocks {
                    info!("{}", stock);
                }
                println!();
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(e) => {
                error!("Ошибка получения из канала: {}", e);
                shutdown.store(true, Ordering::Relaxed);
                break;
            }
        }
    }

    info!("Завершаем работу...");

    receive_handle.join().unwrap();
    Ok(())
}
