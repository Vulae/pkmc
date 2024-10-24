use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use pkmc_defs::packet;
use pkmc_packet::{create_packet_enum, Connection};

use crate::server_state::ServerState;

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

create_packet_enum!(ClientHandshakeWaitingPacket;
    packet::handshake::Handshake, Handshake;
);

create_packet_enum!(ClientHandshakeStatusPacket;
    packet::handshake::StatusRequest, StatusRequest;
    packet::handshake::Ping, Ping;
);

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
        if let Some(raw_packet) = self.connection.recieve()? {
            match self.state {
                ClientHandshakeState::Waiting => {
                    match ClientHandshakeWaitingPacket::try_from(raw_packet)? {
                        ClientHandshakeWaitingPacket::Handshake(handshake) => {
                            self.state = handshake.next_state.try_into()?;
                        }
                    }
                }
                ClientHandshakeState::Closed => unreachable!(),
                ClientHandshakeState::Status => {
                    match ClientHandshakeStatusPacket::try_from(raw_packet)? {
                        ClientHandshakeStatusPacket::StatusRequest(_) => {
                            let server_state = self.server_state.lock().unwrap();
                            self.connection.send(packet::handshake::StatusResponse {
                                version: packet::handshake::StatusResponseVersion {
                                    name: "1.21.1".to_owned(),
                                    protocol: 767,
                                },
                                players: Some(packet::handshake::StatusResponsePlayers {
                                    online: 0,
                                    max: 20,
                                    sample: Vec::new(),
                                }),
                                description: server_state.server_list_text.as_ref().map(|text| {
                                    packet::handshake::StatusResponseDescription {
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
                        ClientHandshakeStatusPacket::Ping(ping) => self.connection.send(ping)?,
                    }
                }
                ClientHandshakeState::Login => unreachable!(),
                ClientHandshakeState::Transfer => unimplemented!(),
            }
        }
        Ok(())
    }
}
