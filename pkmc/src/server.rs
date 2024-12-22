use anyhow::{anyhow, Result};
use pkmc_packet::Connection;
use pkmc_util::IterRetain;
use std::{
    net::{SocketAddr, TcpListener, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{
    client::Client,
    player::{Player, PlayerError},
    server_state::ServerState,
};

#[derive(Debug)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
    address: SocketAddr,
    listener: TcpListener,
    clients: Vec<Client>,
    players: Vec<Player>,
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
            players: Vec::new(),
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

        self.clients
            .retain_returned(|client| !client.client_is_play())
            .try_for_each(|client| {
                let (connection, player_information) = client.into_client_play_state()?;
                println!("Connection {}", player_information.name);
                self.players
                    .push(Player::new(connection, player_information)?);
                Ok::<_, anyhow::Error>(())
            })?;

        self.players.iter_mut().try_for_each(|player| {
            if let Err(err) = player.update() {
                // FIXME: How to display the actual .unwrap() formatted output?
                player.kick(format!("{:?}", err))?;
            }
            Ok::<_, PlayerError>(())
        })?;

        self.players
            .retain_returned(|player| !player.is_closed())
            .for_each(|player| {
                println!("Disconnected {}", player.name());
            });

        Ok(())
    }
}
