use crate::client_registry::{ClientInfo, ClientRegistry};
use anyhow::{Result as AnyResult, anyhow};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

pub struct ClientGuard {
    registry: Arc<Mutex<ClientRegistry>>,
    addr: SocketAddr,
}

impl Drop for ClientGuard {
    fn drop(&mut self) {
        let _ = remove_client(self.addr, self.registry.clone());
    }
}

pub fn add_client_guarded(
    client: ClientInfo,
    registry: Arc<Mutex<ClientRegistry>>,
) -> AnyResult<ClientGuard> {
    let client_addr = client.addr;
    add_client(client, registry.clone())?;
    Ok(ClientGuard {
        registry,
        addr: client_addr,
    })
}

pub(super) fn add_client(
    client: ClientInfo,
    client_registry: Arc<Mutex<ClientRegistry>>,
) -> AnyResult<()> {
    let mut registry = client_registry
        .lock()
        .map_err(|e| anyhow!("Failed to lock client registry: {}", e))?;

    registry
        .add_client(client)
        .map_err(|e| anyhow!("Failed to add client to registry: {}", e))?;
    Ok(())
}

pub(super) fn remove_client(
    udp_addr: SocketAddr,
    client_registry: Arc<Mutex<ClientRegistry>>,
) -> AnyResult<()> {
    let mut registry = client_registry
        .lock()
        .map_err(|e| anyhow!("Failed to lock client registry: {}", e))?;
    registry
        .remove_client(udp_addr)
        .map_err(|e| anyhow!("Failed to add client to registry: {}", e))?;
    Ok(())
}
