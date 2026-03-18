use crate::server::handle_client;
use log::{error, info};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{bounded, unbounded};
use quote_core::StockQuote;
use quote_core::ticker_list::get_tickers_from_txt;
use crate::generators::QuoteGenerator;

mod sender;
mod server;
pub mod generators;

const DEFAULT_TICKERS_PATH: &str = "../../tickers.txt";

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let tickers = get_tickers_from_txt(DEFAULT_TICKERS_PATH)?
        .into_iter()
        .collect();

    let serv_host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let serv_port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let url = format!("{}:{}", serv_host, serv_port);
    let listener = TcpListener::bind(&url)?;
    info!("Server started at {}", url);

    let mut quote_gen = QuoteGenerator::new(tickers);
    let (gen_tx, gen_rx) = bounded(100);
    
    thread::spawn(move || quote_gen.generator_thread(gen_tx));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream, quote_gen.clone()),
            Err(e) => error!("Connection failed: {}!", e),
        }
    }

    Ok(())
}
