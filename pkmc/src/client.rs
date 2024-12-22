use std::sync::{Arc, Mutex};

use pkmc_defs::{packet, Registry};
use pkmc_nbt::NBT;
use pkmc_packet::{
    connection::{self, ConnectionError, ServerboundPacket},
    Connection,
};
use pkmc_util::UUID;
use thiserror::Error;

use crate::server_state::ServerState;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error("Invalid handshake next state {0}")]
    InvalidHandshakeState(i32),
    #[error("Configuration error")]
    ConfigurationError,
    #[error("Client couldn't convert into play state")]
    ClientConversionPlayState,
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
struct ClientLogin {
    server_state: Arc<Mutex<ServerState>>,
    player_information: Option<PlayerInformation>,
    finished_login: bool,
}

impl ClientLogin {
    fn new(server_state: Arc<Mutex<ServerState>>) -> Self {
        Self {
            server_state,
            player_information: None,
            finished_login: false,
        }
    }

    fn update(&mut self, connection: &mut Connection) -> Result<(), ClientError> {
        while let Some(packet) = connection.recieve()? {
            match packet::login::LoginPacket::try_from(&packet)? {
                packet::login::LoginPacket::Hello(hello) => {
                    self.player_information = Some(PlayerInformation {
                        name: hello.name.clone(),
                        uuid: hello.uuid,
                    });

                    let server_state = self.server_state.lock().unwrap();
                    if server_state.compression_level > 0 {
                        connection.send(packet::login::Compression {
                            threshold: server_state.compression_threshold as i32,
                        })?;
                        connection.set_handler(connection::StreamHandler::Zlib(
                            connection::ZlibStreamHandler::new(
                                server_state.compression_threshold,
                                server_state.compression_level,
                            ),
                        ));
                    }

                    connection.send(packet::login::Finished {
                        uuid: hello.uuid,
                        name: hello.name,
                        properties: Vec::new(),
                    })?;
                }
                packet::login::LoginPacket::Acknowledged(_acknowledged) => {
                    self.finished_login = true;
                    break;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientConfigurationFinishState {
    Configuring,
    ConfiguringFinishable,
    WaitingFinish,
    Finished,
}

#[derive(Debug)]
struct ClientConfiguration {
    #[allow(unused)]
    server_state: Arc<Mutex<ServerState>>,
    player_information: PlayerInformation,
    last_packet_time: Option<std::time::Instant>,
    finish_state: ClientConfigurationFinishState,
}

impl ClientConfiguration {
    fn new(
        server_state: Arc<Mutex<ServerState>>,
        player_information: PlayerInformation,
        connection: &mut Connection,
    ) -> Result<Self, ClientError> {
        connection.send(packet::configuration::CustomPayload::Brand(
            "Vulae/pkmc".to_owned(),
        ))?;

        connection.send(packet::configuration::SelectKnownPacks {
            packs: vec![packet::configuration::KnownPack {
                namespace: "minecraft:core".to_owned(),
                id: "".to_owned(),
                version: "1.21".to_owned(),
            }],
        })?;

        Ok(Self {
            server_state,
            player_information,
            last_packet_time: None,
            finish_state: ClientConfigurationFinishState::Configuring,
        })
    }

    fn update(&mut self, connection: &mut Connection) -> Result<(), ClientError> {
        assert_ne!(self.finish_state, ClientConfigurationFinishState::Finished);

        while let Some(packet) = connection.recieve()? {
            let packet = packet::configuration::ConfigurationPacket::try_from(&packet)?;

            match self.finish_state {
                ClientConfigurationFinishState::Configuring
                | ClientConfigurationFinishState::ConfiguringFinishable => match packet {
                    packet::configuration::ConfigurationPacket::CustomPayload(_custom_payload) => {}
                    packet::configuration::ConfigurationPacket::ClientInformation(
                        _client_information,
                    ) => {}
                    packet::configuration::ConfigurationPacket::SelectKnownPacks(
                        _select_known_packs,
                    ) => {
                        let registry = Registry::load();

                        registry
                            .iter_entries()
                            .try_for_each(|(registry_id, entries)| {
                                connection.send(packet::configuration::RegistryData {
                                    registry_id: registry_id.to_owned(),
                                    entries: entries
                                        .iter()
                                        .map(|(id, data)| {
                                            packet::configuration::RegistryDataEntry {
                                                entry_id: id.to_owned(),
                                                data: match NBT::try_from(data.clone()) {
                                                    Ok(nbt) => Ok(Some(nbt)),
                                                    Err(pkmc_nbt::NBTError::JsonConversionEmptyArray) => Ok(None),
                                                    Err(err) => Err(err),
                                                }
                                                // TODO: ERROR HANDLING
                                                .unwrap(),
                                            }
                                        })
                                        .collect::<Vec<_>>(),
                                })?;
                                Ok::<_, ClientError>(())
                            })?;

                        self.finish_state = ClientConfigurationFinishState::ConfiguringFinishable;
                    }
                    packet::configuration::ConfigurationPacket::FinishConfiguration(
                        _finish_configuration,
                    ) => {
                        return Err(ClientError::ConfigurationError);
                    }
                },
                ClientConfigurationFinishState::WaitingFinish => {
                    if let packet::configuration::ConfigurationPacket::FinishConfiguration(
                        _finish_configuration,
                    ) = packet
                    {
                        self.finish_state = ClientConfigurationFinishState::Finished;
                        break;
                    } else {
                        return Err(ClientError::ConfigurationError);
                    }
                }
                _ => {}
            }

            self.last_packet_time = Some(std::time::Instant::now());
        }

        if self.finish_state == ClientConfigurationFinishState::ConfiguringFinishable {
            const FINISH_CONFIGURATION_TIMEOUT: std::time::Duration =
                std::time::Duration::from_millis(100);
            if let Some(last_packet_time) = self.last_packet_time {
                let time = std::time::Instant::now().duration_since(last_packet_time);
                if time >= FINISH_CONFIGURATION_TIMEOUT {
                    self.finish_state = ClientConfigurationFinishState::WaitingFinish;
                    connection.send(packet::configuration::FinishConfiguration)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PlayerInformation {
    pub name: String,
    pub uuid: UUID,
}

#[derive(Debug)]
enum ClientState {
    /// Testing state
    #[allow(unused)]
    Limbo,
    Handshake(ClientHandshake),
    Status(ClientStatus),
    Login(ClientLogin),
    Configuration(ClientConfiguration),
    Play(PlayerInformation),
}

impl ClientState {
    fn update(&mut self, connection: &mut Connection) -> Result<(), ClientError> {
        match self {
            ClientState::Limbo => Ok(()),
            ClientState::Handshake(client_handshake) => client_handshake.update(connection),
            ClientState::Status(client_status) => client_status.update(connection),
            ClientState::Login(client_login) => client_login.update(connection),
            ClientState::Configuration(client_configuration) => {
                client_configuration.update(connection)
            }
            ClientState::Play(..) => Ok(()),
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

        match &self.client_state {
            ClientState::Handshake(ClientHandshake {
                next_state: Some(packet::handshake::IntentionNextState::Status),
            }) => {
                self.client_state = ClientState::Status(ClientStatus {
                    server_state: self.server_state.clone(),
                });
            }
            ClientState::Handshake(ClientHandshake {
                next_state: Some(packet::handshake::IntentionNextState::Login),
            }) => {
                self.client_state = ClientState::Login(ClientLogin::new(self.server_state.clone()));
            }
            ClientState::Login(ClientLogin {
                player_information: Some(player_information),
                finished_login: true,
                ..
            }) => {
                println!("Login from {}", player_information.name);
                self.client_state = ClientState::Configuration(ClientConfiguration::new(
                    self.server_state.clone(),
                    player_information.clone(),
                    &mut self.connection,
                )?);
            }
            ClientState::Configuration(ClientConfiguration {
                player_information,
                finish_state: ClientConfigurationFinishState::Finished,
                ..
            }) => {
                self.client_state = ClientState::Play(player_information.clone());
            }
            _ => {}
        }

        Ok(())
    }

    pub fn client_is_play(&self) -> bool {
        matches!(self.client_state, ClientState::Play(..))
    }

    pub fn into_client_play_state(self) -> Result<(Connection, PlayerInformation), ClientError> {
        match self.client_state {
            ClientState::Play(client_information) => Ok((self.connection, client_information)),
            _ => Err(ClientError::ClientConversionPlayState),
        }
    }
}
