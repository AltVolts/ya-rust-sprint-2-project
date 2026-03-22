use crate::cli::Cli;
use crate::file::read_tickers_file;
use crate::receiver::QuoteReceiver;
use env_logger::{Builder, Env};
use log::{error, info};
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

mod cli;
mod file;
mod receiver;

const MAX_UDP_TIMEOUT: Duration = Duration::from_secs(5);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Builder::from_env(Env::default().default_filter_or("debug")).init();
    let args = Cli::get_args();

    let mut stream = TcpStream::connect(args.server_addr)?;
    // let mut reader = BufReader::new(stream.try_clone()?);

    let tickers_str = read_tickers_file(args.tickers_file)?;

    let command = format!("STREAM udp://{} {}\n", args.udp_port, tickers_str);
    stream.write_all(command.as_bytes())?;

    let quotes_receiver = QuoteReceiver::new(args.udp_port)?;
    let (receive_handle, quotes_rx) = quotes_receiver.start_with_channel();

    loop {
        match quotes_rx.recv() {
            Ok((mut stocks, addr)) => {
                info!("[ОСНОВНОЙ ПОТОК] Получено {} от {}", stocks.len(), addr);
                stocks.sort_by(|a, b| a.ticker.cmp(&b.ticker));
                for stock in stocks {
                    info!("  {}", stock);
                }
            }
            Err(e) => {
                error!("Ошибка получения из канала: {}", e);
                break;
            }
        }
    }

    if let Err(e) = receive_handle.join() {
        error!("Ошибка при ожидании потока-приёмника: {:?}", e);
    }

    Ok(())
}
