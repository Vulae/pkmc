use std::collections::HashMap;

use pkmc_defs::{packet, registry::Registries};
use pkmc_util::{
    nbt::{NBTError, NBT},
    packet::{
        handler::{PacketHandler, ZlibPacketHandler},
        Connection, ConnectionError, ServerboundPacket,
    },
    IdTable, UUID,
};
use thiserror::Error;

const PROTOCOL_VERSION: i32 = 769;
// NOTE: This whole timeout thing is probably dumb, and not the proper way to do this.
const CONFIGURATION_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(
    // NOTE: Probably only running locally, so save us some time :)
    #[cfg(debug_assertions)]
    100,
    #[cfg(not(debug_assertions))]
    1000,
);

#[derive(Error, Debug)]
pub enum ClientHandlerError {
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    NBTError(#[from] NBTError),
    #[error("Invalid protocol version (expected {PROTOCOL_VERSION} | -1, got {0})")]
    InvalidProtocolVersion(i32),
    #[error("Invalid login player")]
    InvalidLoginPlayer,
    #[error("Invalid configuration finalization")]
    InvalidConfigurationFinalization,
}

#[derive(Debug)]
enum ClientHandlerState {
    Closed,
    Handshake,
    Status,
    Login {
        player: Option<(UUID, String)>,
    },
    Configuration {
        player: (UUID, String),
        sent_initial_configuration_packets: bool,
        last_packet_time: std::time::Instant,
        can_finalize: bool,
        sent_finalize_packet: bool,
    },
    Play {
        player: (UUID, String),
    },
}

#[derive(Debug)]
pub struct ClientHandlerPlay {
    pub connection: Connection,
    pub player_id: UUID,
    pub player_name: String,
}

#[derive(Debug)]
pub struct ClientHandler {
    connection: Connection,
    state: ClientHandlerState,

    brand: Option<String>,
    compression: Option<(usize, u32)>,
    status_description: Option<String>,
    status_favicon: Option<String>,
    registries: Option<Registries>,
    tags: Option<HashMap<String, IdTable<String>>>,
}

impl ClientHandler {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            state: ClientHandlerState::Handshake,
            brand: None,
            compression: None,
            status_description: None,
            status_favicon: None,
            registries: None,
            tags: None,
        }
    }

    pub fn with_brand(mut self, brand: impl Into<String>) -> Self {
        self.brand = Some(brand.into());
        self
    }

    /// threshold is number of bytes to compress packet
    /// level is compression level (0..=9, where 0 is no compression)
    pub fn with_compression(mut self, threshold: usize, level: u32) -> Self {
        if (1..=9).contains(&level) {
            self.compression = Some((threshold, level));
        }
        self
    }

    pub fn with_status_description(mut self, description: impl Into<String>) -> Self {
        self.status_description = Some(description.into());
        self
    }

    /// MUST be base64 encoded 64x64 png image.
    pub fn with_status_favicon(mut self, favicon: impl Into<String>) -> Self {
        const BASE64_ENCODED_START: &str = "data:image/png;base64,";
        let str: String = favicon.into();
        self.status_favicon = Some(if str.starts_with(BASE64_ENCODED_START) {
            str
        } else {
            format!("{}{}", BASE64_ENCODED_START, str)
        });
        self
    }

    pub fn with_registies(mut self, registries: impl Into<Registries>) -> Self {
        self.registries = Some(registries.into());
        self
    }

    pub fn with_tags(mut self, tags: impl Into<HashMap<String, IdTable<String>>>) -> Self {
        self.tags = Some(tags.into());
        self
    }

    pub fn into_connection(self) -> Connection {
        self.connection
    }

    pub fn update(&mut self) -> Result<(), ClientHandlerError> {
        if self.connection.is_closed() {
            self.state = ClientHandlerState::Closed;
            return Ok(());
        }

        match self.state {
            ClientHandlerState::Closed => {}
            ClientHandlerState::Handshake => {
                let Some(packet) = self.connection.recieve()? else {
                    return Ok(());
                };
                let intentions = packet::handshake::Intention::packet_raw_read(&packet)?;
                if intentions.protocol_version != -1
                    && intentions.protocol_version != PROTOCOL_VERSION
                {
                    self.state = ClientHandlerState::Closed;
                    return Err(ClientHandlerError::InvalidProtocolVersion(
                        intentions.protocol_version,
                    ));
                }
                match intentions.next_state {
                    packet::handshake::IntentionNextState::Status => {
                        self.state = ClientHandlerState::Status;
                    }
                    packet::handshake::IntentionNextState::Login => {
                        self.state = ClientHandlerState::Login { player: None };
                    }
                    packet::handshake::IntentionNextState::Transfer => unimplemented!(),
                }
            }
            ClientHandlerState::Status => {
                while let Some(packet) = self
                    .connection
                    .recieve_into::<packet::status::StatusPacket>()?
                {
                    match packet {
                        packet::status::StatusPacket::Request(_request) => {
                            self.connection.send(&packet::status::Response {
                                version: packet::status::ResponseVersion {
                                    name: "1.21.4".to_owned(),
                                    protocol: PROTOCOL_VERSION,
                                },
                                players: None,
                                description: self
                                    .status_description
                                    .take()
                                    .map(|s| packet::status::ResponseDescription { text: s }),
                                favicon: self.status_favicon.take(),
                                enforces_secure_chat: false,
                            })?;
                        }
                        packet::status::StatusPacket::Ping(ping) => {
                            self.connection.send(&ping)?;
                            self.state = ClientHandlerState::Closed;
                        }
                    }
                }
            }
            ClientHandlerState::Login { ref mut player } => {
                // TODO: Make this use while loop instead.
                if let Some(packet) = self
                    .connection
                    .recieve_into::<packet::login::LoginPacket>()?
                {
                    match packet {
                        packet::login::LoginPacket::Hello(hello) => {
                            *player = Some((hello.uuid, hello.name.clone()));

                            if let Some((threshold, level)) = self.compression {
                                self.connection.send(&packet::login::Compression {
                                    threshold: threshold as i32,
                                })?;
                                self.connection.set_packet_handler(PacketHandler::Zlib(
                                    ZlibPacketHandler::new(threshold, level),
                                ));
                            }

                            self.connection.send(&packet::login::Finished {
                                uuid: hello.uuid,
                                name: hello.name,
                                properties: Vec::new(),
                            })?;
                        }
                        packet::login::LoginPacket::Acknowledged(_acknowledged) => {
                            self.state = ClientHandlerState::Configuration {
                                player: player
                                    .clone()
                                    .ok_or(ClientHandlerError::InvalidLoginPlayer)?,
                                sent_initial_configuration_packets: false,
                                last_packet_time: std::time::Instant::now(),
                                can_finalize: false,
                                sent_finalize_packet: false,
                            };
                        }
                    }
                }
            }
            ClientHandlerState::Configuration {
                ref player,
                ref mut sent_initial_configuration_packets,
                ref mut last_packet_time,
                ref mut can_finalize,
                ref mut sent_finalize_packet,
            } => {
                if !*sent_finalize_packet {
                    if !*sent_initial_configuration_packets {
                        *sent_initial_configuration_packets = true;

                        if let Some(brand) = self.brand.take() {
                            self.connection
                                .send(&packet::configuration::CustomPayload::Brand(brand))?;
                        }

                        self.connection
                            .send(&packet::configuration::SelectKnownPacks {
                                packs: vec![packet::configuration::KnownPack {
                                    namespace: "minecraft:core".to_owned(),
                                    id: "".to_owned(),
                                    version: "1.21".to_owned(),
                                }],
                            })?;
                    }

                    while let Some(packet) =
                        self.connection
                            .recieve_into::<packet::configuration::ConfigurationPacket>()?
                    {
                        match packet {
                            packet::configuration::ConfigurationPacket::CustomPayload(
                                _custom_payload,
                            ) => {}
                            packet::configuration::ConfigurationPacket::ClientInformation(
                                _client_information,
                            ) => {}
                            packet::configuration::ConfigurationPacket::SelectKnownPacks(
                                _select_known_packs,
                            ) => {
                                // NOTE: This is very very bad and ugly, somehow uglier than myself.
                                // Once registries are actually properly implemented, pretty much all
                                // of this will not be needed.
                                if let Some(registry) = self.registries.take() {
                                    registry
                                    .into_iter()
                                    .try_for_each(|(registry_id, entries)| {
                                        self.connection
                                            .send(&packet::configuration::RegistryData {
                                            registry_id,
                                            entries: entries
                                                .into_iter()
                                                .map(|(entry_id, data)| {
                                                    Ok::<_, ClientHandlerError>(packet::configuration::RegistryDataEntry {
                                                        entry_id,
                                                        data: Some(NBT::try_from(data)?),
                                                    })
                                                })
                                                .collect::<Result<Vec<_>, _>>()?,
                                        })?;
                                        Ok::<_, ClientHandlerError>(())
                                    })?;
                                }

                                if let Some(tags) = self.tags.take() {
                                    self.connection.send(&packet::configuration::UpdateTags {
                                        registries: tags
                                            .into_iter()
                                            .map(|(k, v)| {
                                                (
                                                    k,
                                                    v.into_iter()
                                                        .map(|(k, v)| (k, vec![v]))
                                                        .collect(),
                                                )
                                            })
                                            .collect(),
                                    })?;
                                }

                                *can_finalize = true;
                            }
                            packet::configuration::ConfigurationPacket::FinishConfiguration(
                                _finish_configuration,
                            ) => {
                                return Err(ClientHandlerError::InvalidConfigurationFinalization);
                            }
                        }

                        *last_packet_time = std::time::Instant::now();
                    }

                    if *can_finalize
                        && std::time::Instant::now().duration_since(*last_packet_time)
                            >= CONFIGURATION_TIMEOUT
                    {
                        *sent_finalize_packet = true;
                        self.connection
                            .send(&packet::configuration::FinishConfiguration)?;
                    }
                } else if let Some(packet) =
                    self.connection
                        .recieve_into::<packet::configuration::ConfigurationPacket>()?
                {
                    if !matches!(
                        packet,
                        packet::configuration::ConfigurationPacket::FinishConfiguration(..)
                    ) {
                        return Err(ClientHandlerError::InvalidConfigurationFinalization);
                    }

                    self.state = ClientHandlerState::Play {
                        player: player.clone(),
                    };
                }
            }
            ClientHandlerState::Play { .. } => {}
        }
        Ok(())
    }

    pub fn is_finalized(&self) -> bool {
        matches!(
            self.state,
            ClientHandlerState::Closed | ClientHandlerState::Play { .. }
        )
    }

    pub fn finalized_play_state(self) -> Option<ClientHandlerPlay> {
        match self.state {
            ClientHandlerState::Play {
                player: (player_id, player_name),
            } => Some(ClientHandlerPlay {
                connection: self.connection,
                player_id,
                player_name,
            }),
            _ => None,
        }
    }
}
