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
    clients_handshaker: Vec<ClientHandshake>,
    clients_login: Vec<ClientLogin>,
    clients: Vec<Client>,
}

impl Server {
    pub fn new<S: ToSocketAddrs>(address: S, state: ServerState) -> Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            listener,
            clients_handshaker: Vec::new(),
            clients_login: Vec::new(),
            clients: Vec::new(),
        })
    }

    fn handle_clients_handshakers(&mut self) -> Result<()> {
        while let Ok((stream, _)) = self.listener.accept() {
            let connection = Connection::new(stream)?;
            self.clients_handshaker
                .push(ClientHandshake::new(self.state.clone(), connection));
        }

        // TODO: For each handshaker, try updating until state is either closed or login.
        self.clients_handshaker
            .iter_mut()
            .map(|handshaker| handshaker.update())
            .collect::<Result<Vec<_>, _>>()?;

        self.clients_handshaker
            .retain(|handshaker| handshaker.state() != ClientHandshakeState::Closed);

        for i in (0..self.clients_handshaker.len()).rev() {
            if self.clients_handshaker[i].state() == ClientHandshakeState::Login {
                let handshaker = self.clients_handshaker.remove(i);
                let login_player =
                    ClientLogin::new(self.state.clone(), handshaker.into_connection());
                self.clients_login.push(login_player);
            }
        }

        Ok(())
    }

    pub fn handle_clients_login(&mut self) -> Result<()> {
        self.clients_login
            .iter_mut()
            .map(|login_player| login_player.update())
            .collect::<Result<Vec<_>, _>>()?;

        for i in (0..self.clients_login.len()).rev() {
            if self.clients_login[i].state() == ClientLoginState::Play {
                let login_player = self.clients_login.remove(i);
                let player = Client::initialize(self.state.clone(), login_player)?;
                self.clients.push(player);
            }
        }

        Ok(())
    }

    pub fn handle_clients(&mut self) -> Result<()> {
        self.clients.iter_mut().for_each(|client| {
            if let Err(err) = client.update() {
                println!("Error while updating client {}: {:#?}", client.name(), err);
            }
        });

        self.clients.retain(|client| {
            if client.disconnected() {
                println!("{} disconnected", client.name());
            }
            !client.disconnected()
        });

        Ok(())
    }

    pub fn step(&mut self) -> Result<()> {
        self.handle_clients_handshakers()?;
        self.handle_clients_login()?;
        self.handle_clients()?;

        Ok(())
    }
}
