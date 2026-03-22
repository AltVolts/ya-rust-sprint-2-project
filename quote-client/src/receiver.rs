use anyhow::Result as AnyhowResult;
use log::{error, info};
use quote_core::{StockQuote, deserialize_quotes};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

type Stocks = Vec<StockQuote>;

const SOCKET_POLL_TIMEOUT_MS: u64 = 2000;

pub(crate) struct QuoteReceiver {
    socket: UdpSocket,
    shutdown: Arc<AtomicBool>,
}

impl QuoteReceiver {
    pub fn new(bind_addr: SocketAddr, shutdown: Arc<AtomicBool>) -> AnyhowResult<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        info!("Udp Receiver starts at {}\n", bind_addr);
        Ok(Self { socket, shutdown })
    }
    pub fn get_socket_clone(&self) -> Arc<UdpSocket> {
        Arc::new(self.socket.try_clone().expect("Failed to clone UDP socket"))
    }

    pub fn start_with_channel_and_server_addr(
        self,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Receiver<(Stocks, SocketAddr)>,
        mpsc::Receiver<SocketAddr>,
    ) {
        let (tx_quotes, rx_quotes) = mpsc::channel();
        let (tx_addr, rx_addr) = mpsc::channel();
        let handle = thread::spawn(move || {
            if let Err(e) = self.receive_loop_with_channel_and_addr(tx_quotes, tx_addr) {
                error!("Ошибка в receive_loop_with_channel_and_addr: {}", e);
            }
        });
        (handle, rx_quotes, rx_addr)
    }

    fn receive_loop_with_channel_and_addr(
        self,
        tx_quotes: mpsc::Sender<(Stocks, SocketAddr)>,
        tx_addr: mpsc::Sender<SocketAddr>,
    ) -> AnyhowResult<()> {
        self.socket
            .set_read_timeout(Some(Duration::from_millis(SOCKET_POLL_TIMEOUT_MS)))?;
        let mut buf = [0u8; 1024];
        let mut addr_sent = false;

        loop {
            if self.shutdown.load(Ordering::Relaxed) {
                info!("Получен сигнал остановки, завершаем поток приёма");
                break;
            }

            match self.socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => {
                    if !addr_sent {
                        if tx_addr.send(src_addr).is_err() {
                            info!("Канал адреса закрыт, завершение потока приёма");
                            break;
                        }
                        addr_sent = true;
                    }

                    match deserialize_quotes(&buf[..size]) {
                        Ok(quotes) => {
                            if tx_quotes.send((quotes, src_addr)).is_err() {
                                info!("Канал котировок закрыт, завершение потока приёма");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Ошибка десериализации: {}", e);
                        }
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    continue;
                }
                Err(e) => {
                    error!("Ошибка получения данных: {}", e);
                    self.shutdown.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        Ok(())
    }
}
