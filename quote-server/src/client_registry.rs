use anyhow::Result;
use quote_core::TickerPrices;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::Instant;

pub type QuoteSender = std::sync::mpsc::Sender<TickerPrices>;

type TickerSet = HashSet<String>;

pub struct ClientInfo {
    addr: SocketAddr,
    last_ping: Instant,
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

    pub fn add_client(&mut self, client: ClientInfo) -> Result<()> {
        let client_addr = client.addr.clone();
        self.clients
            .insert(client.addr, client)
            .ok_or_else(|| anyhow::anyhow!("Client {:?} already exists", client_addr))?;
        Ok(())
    }

    pub fn remove_client(&mut self, client_addr: SocketAddr) -> Result<()> {
        self.clients
            .remove(&client_addr)
            .ok_or_else(|| anyhow::anyhow!("Client {:?} not exists", client_addr))?;
        Ok(())
    }

    pub fn broadcast(&self, prices: &TickerPrices) -> Result<()> {
        for client in self.clients.values() {
            client.sender.send(prices.clone())?;
        }
        Ok(())
    }
}
