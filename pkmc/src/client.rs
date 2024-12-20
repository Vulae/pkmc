use std::sync::{Arc, Mutex};

use pkmc_defs::packet;
use pkmc_packet::{
    connection::{ConnectionError, ServerboundPacket},
    Connection,
};
use thiserror::Error;

use crate::server_state::ServerState;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Invalid handshake next state {0}")]
    InvalidHandshakeState(i32),
    #[error("{0:?}")]
    ConnectionError(#[from] ConnectionError),
}

#[derive(Debug)]
struct ClientHandshake {
    next_state: Option<packet::handshake::IntentionNextState>,
}

impl ClientHandshake {
    fn update(&mut self, connection: &mut Connection) -> Result<(), ClientError> {
        let Some(packet) = connection.recieve()? else {
            return Ok(());
        };
        let intentions = packet::handshake::Intention::packet_raw_read(&packet)?;
        self.next_state = Some(intentions.next_state);
        Ok(())
    }
}

#[derive(Debug)]
struct ClientStatus {
    server_state: Arc<Mutex<ServerState>>,
}

impl ClientStatus {
    fn update(&mut self, connection: &mut Connection) -> Result<(), ClientError> {
        while let Some(packet) = connection.recieve()? {
            match packet::status::StatusPacket::try_from(&packet)? {
                packet::status::StatusPacket::Request(_request) => {
                    let server_state = self.server_state.lock().unwrap();
                    connection.send(packet::status::Response {
                        version: packet::status::ResponseVersion {
                            name: "1.21.4".to_owned(),
                            protocol: 769,
                        },
                        players: None,
                        description: server_state
                            .server_list_text
                            .clone()
                            .map(|text| packet::status::ResponseDescription { text }),
                        favicon: server_state.server_list_icon.clone(),
                        enforces_secure_chat: false,
                    })?;
                }
                packet::status::StatusPacket::Ping(ping) => {
                    connection.send(ping)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ClientState {
    Handshake(ClientHandshake),
    Status(ClientStatus),
}

impl ClientState {
    fn update(&mut self, connection: &mut Connection) -> Result<(), ClientError> {
        match self {
            ClientState::Handshake(client_handshake) => client_handshake.update(connection),
            ClientState::Status(client_status) => client_status.update(connection),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    connection: Connection,
    server_state: Arc<Mutex<ServerState>>,
    client_state: ClientState,
}

impl Client {
    pub fn new(connection: Connection, server_state: Arc<Mutex<ServerState>>) -> Self {
        Self {
            connection,
            server_state,
            client_state: ClientState::Handshake(ClientHandshake { next_state: None }),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.connection.is_closed()
    }

    pub fn update(&mut self) -> Result<(), ClientError> {
        self.client_state.update(&mut self.connection)?;

        if let ClientState::Handshake(ClientHandshake {
            next_state: Some(packet::handshake::IntentionNextState::Status),
        }) = self.client_state
        {
            self.client_state = ClientState::Status(ClientStatus {
                server_state: self.server_state.clone(),
            });
        }

        Ok(())
    }
}
