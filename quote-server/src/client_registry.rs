use anyhow::Result as AnyhowResult;
use log::info;
use quote_core::TickerPrices;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

pub type QuoteSender = std::sync::mpsc::Sender<TickerPrices>;

pub struct ClientInfo {
    pub addr: SocketAddr,
    pub last_ping: Instant,
    pub sender: QuoteSender,
}

impl ClientInfo {
    pub fn new(addr: SocketAddr, sender: QuoteSender) -> Self {
        Self {
            addr,
            last_ping: Instant::now(),
            sender,
        }
    }

    pub fn update_ping(&mut self) {
        self.last_ping = Instant::now();
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_ping.elapsed() > timeout
    }
}

pub struct ClientRegistry {
    clients: HashMap<SocketAddr, ClientInfo>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        ClientRegistry {
            clients: HashMap::new(),
        }
    }

    pub fn add_client(&mut self, client: ClientInfo) -> AnyhowResult<()> {
        let client_addr = client.addr;
        match self.clients.entry(client_addr) {
            Entry::Occupied(_) => Err(anyhow::anyhow!("Client {:?} already exists", client_addr)),
            Entry::Vacant(entry) => {
                entry.insert(client);
                info!("Client {:?} added to registry", client_addr);
                Ok(())
            }
        }
    }

    pub fn remove_client(&mut self, client_addr: SocketAddr) -> AnyhowResult<()> {
        self.clients
            .remove(&client_addr)
            .ok_or_else(|| anyhow::anyhow!("Client {:?} not exists", client_addr))?;
        info!("Client {:?} removed from registry", client_addr);
        Ok(())
    }

    pub fn broadcast(&self, prices: &TickerPrices) -> AnyhowResult<()> {
        for client in self.clients.values() {
            client.sender.send(prices.clone())?;
        }
        Ok(())
    }

    pub fn get_mut(&mut self, addr: &SocketAddr) -> Option<&mut ClientInfo> {
        self.clients.get_mut(addr)
    }

    pub fn get(&self, addr: &SocketAddr) -> Option<&ClientInfo> {
        self.clients.get(addr)
    }
}
