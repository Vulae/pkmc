use anyhow::{anyhow, Result};
use pkmc_packet::Connection;
use std::{
    net::{SocketAddr, TcpListener, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{client::Client, server_state::ServerState};

#[derive(Debug)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
    address: SocketAddr,
    listener: TcpListener,
    clients: Vec<Client>,
    // players: Vec<Player>,
}

impl Server {
    pub fn new<S: ToSocketAddrs>(address: S, state: ServerState) -> Result<Self> {
        let address = address
            .to_socket_addrs()?
            .next()
            .ok_or(anyhow!("Failed to parse socket address"))?;
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            address,
            state: Arc::new(Mutex::new(state)),
            listener,
            clients: Vec::new(),
            // players: Vec::new(),
        })
    }

    pub fn ip(&self) -> &SocketAddr {
        &self.address
    }

    pub fn step(&mut self) -> Result<()> {
        while let Ok((stream, _)) = self.listener.accept() {
            let connection = Connection::new(stream)?;
            self.clients
                .push(Client::new(connection, self.state.clone()));
        }

        self.clients
            .iter_mut()
            .try_for_each(|client| client.update())?;

        self.clients.retain(|client| !client.is_closed());

        Ok(())
    }
}
