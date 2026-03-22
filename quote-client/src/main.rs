use crate::cli::Cli;
use crate::file::read_tickers_file;
use crate::receiver::QuoteReceiver;
use env_logger::{Builder, Env};
use log::{error, info};
use quote_core::StockQuote;
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

mod cli;
mod file;
mod keep_alive;
mod receiver;

const POLL_INTERVAL_MS: u64 = 2000;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Builder::from_env(Env::default().default_filter_or("debug")).init();
    let args = Cli::get_args();

    ctrlc::set_handler(|| {
        SHUTDOWN.store(true, Ordering::Relaxed);
        info!("Получен сигнал завершения, начинаем остановку...");
    })?;

    let mut stream = TcpStream::connect(args.server_addr)?;
    // let mut reader = BufReader::new(stream.try_clone()?);

    let tickers_str = read_tickers_file(args.tickers_file)?;

    let command = format!("STREAM udp://{} {}\n", args.udp_port, tickers_str);
    stream.write_all(command.as_bytes())?;
    stream.flush()?;
    if let Err(e) = stream.shutdown(Shutdown::Both) {
        error!("Ошибка при закрытии TCP-соединения: {}", e);
    }

    let quotes_receiver = QuoteReceiver::new(args.udp_port)?;
    let (receive_handle, quotes_rx) = quotes_receiver.start_with_channel();

    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
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
                    info!("{}", stock.to_string());
                }
                print!("\n");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                continue;
            }
            Err(e) => {
                error!("Ошибка получения из канала: {}", e);
                SHUTDOWN.store(true, Ordering::Relaxed);
                break;
            }
        }
    }

    info!("Завершаем работу...");

    receive_handle.join().unwrap();
    Ok(())
}
