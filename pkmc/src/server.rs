use anyhow::{anyhow, Result};
use pkmc_defs::text_component::{self, TextComponent};
use pkmc_packet::Connection;
use pkmc_util::IterRetain;
use std::{
    net::{SocketAddr, TcpListener, ToSocketAddrs},
    sync::{Arc, RwLock},
};

use crate::{
    client::Client,
    player::{Player, PlayerError},
    server_state::ServerState,
};

#[derive(Debug)]
pub struct Server {
    state: Arc<RwLock<ServerState>>,
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
        println!("Server running at {}", address);
        listener.set_nonblocking(true)?;
        Ok(Self {
            address,
            state: Arc::new(RwLock::new(state)),
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
                self.players.push(Player::new(
                    connection,
                    self.state.clone(),
                    player_information,
                )?);
                Ok::<_, anyhow::Error>(())
            })?;

        self.players.iter_mut().try_for_each(|player| {
            if let Err(err) = player.update() {
                player.kick(
                    TextComponent::empty()
                        .with_child(|child| child.with_content("Server Player Error"))
                        .with_child(|child| {
                            child
                                .with_content(format!("\n\n{}", err))
                                .with_color(text_component::Color::RED)
                        }),
                )?;
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
