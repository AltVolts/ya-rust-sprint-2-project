use crate::client_registry::ClientRegistry;
use log::{debug, info, warn};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;

const PING_MESSAGE: &[u8] = b"PING";

pub fn start_ping_listener(
    socket: Arc<UdpSocket>,
    registry: Arc<Mutex<ClientRegistry>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = [0; 1024];
        info!("Ping listener started");
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    if &buf[..len] == PING_MESSAGE {
                        let mut reg = registry.lock().unwrap();
                        if let Some(client) = reg.get_mut(&src) {
                            client.update_ping();
                        } else {
                            debug!("Received ping from unknown client: {}", src);
                        }
                    } else {
                        debug!("Received non-ping message from {}: {:?}", src, &buf[..len]);
                    }
                }
                Err(e) => {
                    warn!("Error receiving UDP packet: {}", e);
                }
            }
        }
    })
}
