use crate::server::handle_client;
use log::{error, info};
use quote_core::QuoteGenerator;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

mod sender;
mod server;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let serv_host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let serv_port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let url = format!("http://{}:{}", serv_host, serv_port);
    let listener = TcpListener::bind(&url)?;
    info!("Server started at {}", url);

    let quote_gen = Arc::new(Mutex::new(QuoteGenerator::new()?));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream, quote_gen.clone()),
            Err(e) => error!("Connection failed: {}!", e),
        }
    }

    Ok(())
}
