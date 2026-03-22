use crate::SHUTDOWN;
use anyhow::Result as AnyhowResult;
use log::{error, info};
use quote_core::{StockQuote, deserialize_quotes};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

type Stocks = Vec<StockQuote>;

const SOCKET_POLL_TIMEOUT_MS: u64 = 2000;

pub(crate) struct QuoteReceiver {
    socket: UdpSocket,
}

impl QuoteReceiver {
    pub fn new(bind_addr: SocketAddr) -> AnyhowResult<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        info!("Udp Receiver starts at {}\n", bind_addr);
        Ok(Self { socket })
    }

    pub fn start_with_channel(
        self,
    ) -> (
        thread::JoinHandle<()>,
        mpsc::Receiver<(Stocks, std::net::SocketAddr)>,
    ) {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            if let Err(e) = self.receive_loop_with_channel(tx) {
                error!("Ошибка в receive_loop_with_channel: {}", e);
            }
        });

        (handle, rx)
    }

    fn receive_loop_with_channel(self, tx: mpsc::Sender<(Stocks, SocketAddr)>) -> AnyhowResult<()> {
        self.socket
            .set_read_timeout(Some(Duration::from_millis(SOCKET_POLL_TIMEOUT_MS)))?;
        let mut buf = [0u8; 1024];

        loop {
            if SHUTDOWN.load(Ordering::Relaxed) {
                info!("Получен сигнал остановки, завершаем поток приёма");
                break;
            }

            match self.socket.recv_from(&mut buf) {
                Ok((size, src_addr)) => match deserialize_quotes(&buf[..size]) {
                    Ok(quotes) => {
                        if tx.send((quotes, src_addr)).is_err() {
                            info!("Канал закрыт, завершение потока приёма");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Ошибка десериализации: {}", e);
                    }
                },
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    continue;
                }
                Err(e) => {
                    error!("Ошибка получения данных: {}", e);
                    SHUTDOWN.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
        Ok(())
    }
}
