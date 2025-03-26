use std::collections::HashMap;

use pkmc_defs::{packet, registry::Registries};
use pkmc_util::{
    IdTable, UUID,
    connection::{
        Connection, ConnectionEncryption, ConnectionError, PacketHandler, ServerboundPacket,
    },
    crypto::{MinecraftSha1, rsa_encode_public_key},
    nbt::{NBT, NBTError},
};
use serde::Deserialize;
use thiserror::Error;

const PROTOCOL_VERSION: i32 = pkmc_generated::packet::PROTOCOL_VERSION;
// NOTE: This whole timeout thing is probably dumb, and not the proper way to do this.
const CONFIGURATION_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

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
    #[error(transparent)]
    RsaError(#[from] rsa::Error),
    #[error("RSA Verify token mismatch")]
    VerifyTokenMismatch,
    #[error("Authentication information doesn't match")]
    AuthMismatch,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum ClientHandlerState {
    Closed,
    Handshake,
    Status,
    Login {
        player: Option<(UUID, String)>,
        private_key: rsa::RsaPrivateKey,
        verify_token: [u8; 64],
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
    online: bool,
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
            online: false,
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

    pub fn with_online(mut self, online: bool) -> Self {
        self.online = online;
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
                        self.state = ClientHandlerState::Login {
                            player: None,
                            private_key: rsa::RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 1024)?,
                            verify_token: rand::random(),
                        };
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
                                    name: pkmc_generated::consts::VERSION_STR.to_owned(),
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
            ClientHandlerState::Login {
                ref mut player,
                ref private_key,
                ref verify_token,
            } => {
                // TODO: Make this use while loop instead.
                if let Some(packet) = self
                    .connection
                    .recieve_into::<packet::login::LoginPacket>()?
                {
                    match packet {
                        packet::login::LoginPacket::Hello(hello) => {
                            *player = Some((hello.uuid, hello.name));

                            self.connection.send(&packet::login::EncryptionRequest {
                                server_id: "".to_string(),
                                public_key: pkmc_util::crypto::rsa_encode_public_key(
                                    &private_key.to_public_key(),
                                ),
                                verify_token: verify_token.to_vec().into_boxed_slice(),
                                should_authenticate: self.online,
                            })?;
                        }
                        packet::login::LoginPacket::EncryptionResponse(encryption_response) => {
                            let shared_secret = private_key.decrypt(
                                rsa::Pkcs1v15Encrypt,
                                &encryption_response.shared_secret,
                            )?;

                            if private_key
                                .decrypt(rsa::Pkcs1v15Encrypt, &encryption_response.verify_token)?
                                != verify_token
                            {
                                return Err(ClientHandlerError::VerifyTokenMismatch);
                            }

                            self.connection.set_connection_encryption(
                                ConnectionEncryption::new_aes(
                                    &shared_secret.clone().try_into().unwrap(),
                                )
                                .map_err(ConnectionError::EncryptionError)?,
                            );

                            // TODO: Non-blocking.
                            let (uuid, name, properties) = if self.online {
                                #[derive(Debug, Deserialize)]
                                struct SessionMinecraftJoinedProperty {
                                    name: String,
                                    value: String,
                                    signature: String,
                                }

                                #[derive(Debug, Deserialize)]
                                struct SessionMinecraftJoined {
                                    id: String,
                                    name: String,
                                    #[serde(default)]
                                    properties: Vec<SessionMinecraftJoinedProperty>,
                                }

                                let (uuid, name) = player.clone().unwrap();

                                let server_id = {
                                    let mut hasher = MinecraftSha1::default();
                                    hasher.update(""); // Minecraft server ID
                                    hasher.update(&shared_secret);
                                    hasher.update(rsa_encode_public_key(
                                        &private_key.to_public_key(),
                                    ));
                                    hasher.finalize()
                                };

                                let auth: SessionMinecraftJoined =
                                    reqwest::blocking::get(reqwest::Url::parse_with_params(
                                    "https://sessionserver.mojang.com/session/minecraft/hasJoined",
                                    &[("username", &name), ("serverId", &server_id)],
                                )
                                .unwrap()).unwrap().json().unwrap();

                                let parsed_uuid = UUID::try_from(auth.id.as_ref()).unwrap();

                                if parsed_uuid != uuid || name != auth.name {
                                    return Err(ClientHandlerError::AuthMismatch);
                                }

                                println!("Auth from: {} {}", auth.name, parsed_uuid);

                                (
                                    parsed_uuid,
                                    auth.name,
                                    // FIXME: Skins dont work :(
                                    auth.properties
                                        .into_iter()
                                        .map(|prop| packet::login::FinishedProperty {
                                            name: prop.name,
                                            value: prop.value,
                                            signature: Some(prop.signature),
                                        })
                                        .collect(),
                                )
                            } else {
                                let (uuid, name) = player.clone().unwrap();
                                (uuid, name, Vec::new())
                            };

                            if let Some((threshold, compression_level)) = self.compression {
                                self.connection.send(&packet::login::Compression {
                                    threshold: threshold as i32,
                                })?;
                                self.connection.set_packet_handler(
                                    PacketHandler::new_zlib(threshold, compression_level)
                                        .map_err(ConnectionError::HandlerError)?,
                                );
                            }

                            self.connection.send(&packet::login::Finished {
                                uuid,
                                name,
                                properties,
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
