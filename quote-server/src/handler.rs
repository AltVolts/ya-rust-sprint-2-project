use super::handler::client::add_client_guarded;
use super::handler::tcp_input::handle_tcp_input;
use crate::client_registry::{ClientInfo, ClientRegistry};
use anyhow::{Result as AnyhowResult, anyhow};
use log::{debug, warn};
use quote_core::{StockQuote, TickerPrices, serialize_quotes};
use std::collections::HashSet;
use std::net::{TcpStream, UdpSocket};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod client;
mod tcp_input;

const PING_TIMEOUT: Duration = Duration::from_secs(5);
const GENERATION_MAX_TIMEOUT: u64 = 500;

/// Handle client tcp connections. Receives quotes from quote generator via channel and send quotes to client via udp socket
pub(crate) fn handle_client(
    stream: TcpStream,
    client_registry: Arc<Mutex<ClientRegistry>>,
    socket: Arc<UdpSocket>,
) -> AnyhowResult<()> {
    let (udp_addr, tickers_set) = handle_tcp_input(stream)?;

    let (tx, rx) = std::sync::mpsc::channel();
    let client = ClientInfo::new(udp_addr, tx);
    let _guard = add_client_guarded(client, client_registry.clone())?;

    let mut failed_count = 0;
    loop {
        if failed_count > 10 {
            return Err(anyhow!("Too many handle client errors"));
        }

        let ticker_prices = match rx.recv_timeout(Duration::from_millis(GENERATION_MAX_TIMEOUT)) {
            Ok(ticker_prices) => ticker_prices,
            Err(RecvTimeoutError::Timeout) => {
                debug!("Reached timeout of generation");
                let expired = {
                    let reg = client_registry.lock().unwrap();
                    reg.get(&udp_addr)
                        .map(|c| c.is_expired(PING_TIMEOUT))
                        .unwrap_or(true)
                };
                if expired {
                    debug!("Client {} timed out, stopping stream", udp_addr);
                    break;
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => {
                warn!("Channel disconnected, stopping client stream");
                break;
            }
        };

        let filtered_prices = filter_prices(ticker_prices, &tickers_set);
        if filtered_prices.is_empty() {
            continue;
        }

        let encoded = match serialize_quotes(filtered_prices) {
            Ok(encoded) => encoded,
            Err(e) => {
                failed_count += 1;
                warn!("Failed to serialize ticker prices: {}", e);
                continue;
            }
        };
        match socket.send_to(&encoded, udp_addr) {
            Ok(_) => {
                failed_count = 0;
            }
            Err(e) => {
                failed_count += 1;
                warn!("Failed to send ticker prices: {}", e);
                continue;
            }
        }
    }
    Ok(())
}

/// Filter generator quote prices according to tickers list from client STREAM command
fn filter_prices(prices: TickerPrices, tickers_set: &HashSet<String>) -> Vec<StockQuote> {
    prices
        .into_iter()
        .filter(|(ticker, _)| tickers_set.contains(ticker))
        .map(|(_, stock)| stock)
        .collect()
}
