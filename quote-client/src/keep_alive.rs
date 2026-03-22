use log::{debug, error, info};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const PING_INTERVAL: Duration = Duration::from_secs(2);
const PING_MESSAGE: &[u8] = b"PING";

pub fn start_ping_sender(
    client_udp: Arc<UdpSocket>,
    server_addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        info!(
            "Ping sender started, sending to {} from local port {}",
            server_addr,
            client_udp.local_addr().map(|a| a.port()).unwrap_or(0)
        );

        while !shutdown.load(Ordering::Relaxed) {
            if let Err(e) = client_udp.send_to(PING_MESSAGE, server_addr) {
                error!("Failed to send ping: {}", e);
            } else {
                debug!("Ping sent to {}", server_addr);
            }
            thread::sleep(PING_INTERVAL);
        }
        info!("Ping sender stopped");
    })
}
