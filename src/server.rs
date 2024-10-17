use anyhow::Result;
use std::{
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, Mutex},
};

use crate::{
    client::Client,
    client_handshake::{ClientHandshake, ClientHandshakeState},
    client_login::{ClientLogin, ClientLoginState},
    connection::Connection,
    server_state::ServerState,
};

#[derive(Debug)]
pub struct Server {
    state: Arc<Mutex<ServerState>>,
    listener: TcpListener,
    handshakers: Vec<ClientHandshake>,
    login_players: Vec<ClientLogin>,
    players: Vec<Client>,
}

impl Server {
    pub fn new<S: ToSocketAddrs>(address: S, state: ServerState) -> Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            listener,
            handshakers: Vec::new(),
            login_players: Vec::new(),
            players: Vec::new(),
        })
    }

    fn handle_handshakers(&mut self) -> Result<()> {
        while let Ok((stream, _)) = self.listener.accept() {
            let connection = Connection::new(stream)?;
            self.handshakers
                .push(ClientHandshake::new(self.state.clone(), connection));
        }

        // TODO: For each handshaker, try updating until state is either closed or login.
        self.handshakers
            .iter_mut()
            .map(|handshaker| handshaker.update())
            .collect::<Result<Vec<_>, _>>()?;

        self.handshakers
            .retain(|handshaker| handshaker.state() != ClientHandshakeState::Closed);

        for i in (0..self.handshakers.len()).rev() {
            if self.handshakers[i].state() == ClientHandshakeState::Login {
                let handshaker = self.handshakers.remove(i);
                let login_player = ClientLogin::new(handshaker.into_connection());
                self.login_players.push(login_player);
            }
        }

        Ok(())
    }

    pub fn handle_login_players(&mut self) -> Result<()> {
        self.login_players
            .iter_mut()
            .map(|login_player| login_player.update())
            .collect::<Result<Vec<_>, _>>()?;

        for i in (0..self.login_players.len()).rev() {
            if self.login_players[i].state() == ClientLoginState::Play {
                let login_player = self.login_players.remove(i);
                let player = Client::initialize(login_player)?;
                self.players.push(player);
            }
        }

        Ok(())
    }

    pub fn handle_players(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn step(&mut self) -> Result<()> {
        self.handle_handshakers()?;
        self.handle_login_players()?;
        self.handle_players()?;

        Ok(())
    }
}
