use std::sync::{Arc, Mutex};

use crate::{
    connection::Connection,
    packet::{self, reader::PacketReader, Packet as _},
    server_state::ServerState,
};
use anyhow::{anyhow, Result};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ClientHandshakeState {
    Waiting,  // If waiting for new state
    Closed,   // If handshaker should be immediately closed
    Status,   // Request server status state
    Login,    // Login state
    Transfer, // File transfer (Resource pack)
}

impl TryFrom<i32> for ClientHandshakeState {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Err(anyhow!("HandshakerState should never try from 0")),
            1 => Ok(ClientHandshakeState::Status),
            2 => Ok(ClientHandshakeState::Login),
            3 => Ok(ClientHandshakeState::Transfer),
            _ => Err(anyhow!("HandshakerState unknown value {}", value)),
        }
    }
}

#[derive(Debug)]
pub struct ClientHandshake {
    server_state: Arc<Mutex<ServerState>>,
    connection: Connection,
    pub state: ClientHandshakeState,
}

impl ClientHandshake {
    pub fn new(server_state: Arc<Mutex<ServerState>>, connection: Connection) -> Self {
        Self {
            server_state,
            connection,
            state: ClientHandshakeState::Waiting,
        }
    }

    pub fn into_connection(self) -> Connection {
        self.connection
    }

    pub fn state(&self) -> ClientHandshakeState {
        self.state
    }

    pub fn update(&mut self) -> Result<()> {
        if let Some((id, data)) = self.connection.recieve()? {
            let mut reader = PacketReader::new(std::io::Cursor::new(data.as_ref()));

            match id {
                0 => match self.state {
                    ClientHandshakeState::Waiting => {
                        let handshake = packet::server_list::Handshake::packet_read(&mut reader)?;
                        self.state = handshake.next_state.try_into()?;
                    }
                    ClientHandshakeState::Status => {
                        let server_state = self.server_state.lock().unwrap();
                        self.connection.send(packet::server_list::StatusResponse {
                            version: packet::server_list::StatusResponseVersion {
                                name: "1.21.1".to_string(),
                                protocol: 767,
                            },
                            players: Some(packet::server_list::StatusResponsePlayers {
                                online: 0,
                                max: 20,
                                sample: Vec::new(),
                            }),
                            description: server_state.server_list_text.as_ref().map(|text| {
                                packet::server_list::StatusResponseDescription {
                                    text: text.to_owned(),
                                }
                            }),
                            favicon: server_state
                                .server_list_icon
                                .as_ref()
                                .map(|icon| format!("data:image/png;base64,{}", icon)),
                            enforces_secure_chat: false,
                        })?;
                    }
                    ClientHandshakeState::Login => panic!(),
                    ClientHandshakeState::Transfer => panic!(),
                    _ => panic!(),
                },
                1 => {
                    self.connection
                        .send(packet::server_list::Ping::packet_read(&mut reader)?)?;
                    self.state = ClientHandshakeState::Closed;
                }
                _ => panic!(),
            }
        }
        Ok(())
    }
}
