use crate::client_registry::ClientRegistry;
use crate::generators::QuoteGenerator;
use crate::handler::handle_client;
use crate::keep_alive::start_ping_listener;
use env_logger::{Builder, Env};
use log::{error, info};
use quote_core::ticker_list::get_tickers_from_txt;
use std::net::{TcpListener, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;

mod client_registry;
pub mod generators;
mod handler;
mod keep_alive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    Builder::from_env(Env::default().default_filter_or("debug")).init();

    let serv_host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let serv_port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let tickers_path =
        std::env::var("TICKERS_PATH").unwrap_or_else(|_| "../tickers.txt".to_string());

    let tickers = get_tickers_from_txt(tickers_path.as_str())?
        .into_iter()
        .collect();

    let url = format!("{}:{}", serv_host, serv_port);
    let listener = TcpListener::bind(&url)?;
    info!("Server started at {}", url);

    // Bind udp socket for quotes senders client threads and for keep-alive thread
    let udp_socket = UdpSocket::bind("0.0.0.0:0")?;
    let udp_port = udp_socket.local_addr()?.port();
    info!("UDP socket bound to port {}", udp_port);
    let udp_socket = Arc::new(udp_socket);

    let client_registry = Arc::new(Mutex::new(ClientRegistry::new()));

    let gen_registry = client_registry.clone();
    let gen_handle = thread::spawn(move || {
        let mut quote_gen = QuoteGenerator::new(gen_registry, tickers);
        if let Err(e) = quote_gen.start_generation() {
            error!("Generator stopped with error: {}", e);
        }
    });

    let ping_registry = client_registry.clone();
    let ping_socket = udp_socket.clone();
    let _ping_handle = start_ping_listener(ping_socket, ping_registry);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = client_registry.clone();
                let udp = udp_socket.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_client(stream, registry, udp) {
                        error!("Error while handling client: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Connection failed: {}!", e)
            }
        }
    }

    gen_handle.join().unwrap();
    Ok(())
}
