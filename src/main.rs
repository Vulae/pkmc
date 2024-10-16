pub mod connection;
pub mod nbt;
pub mod packet;
pub mod uuid;

use std::{
    collections::HashMap,
    net::{TcpListener, ToSocketAddrs},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use base64::Engine as _;
use connection::Connection;
use nbt::NBT;
use packet::{reader::PacketReader, Packet};
use uuid::UUID;

static SERVER_ICON: &[u8] = include_bytes!("../server_icon.png");

#[derive(Debug, PartialEq, Clone, Copy)]
enum HandshakerState {
    Waiting,  // If waiting for new state
    Closed,   // If handshaker should be immediately closed
    Status,   // Request server status state
    Login,    // Login state
    Transfer, // File transfer (Resource pack)
}

impl TryFrom<i32> for HandshakerState {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Err(anyhow!("HandshakerState should never try from 0")),
            1 => Ok(HandshakerState::Status),
            2 => Ok(HandshakerState::Login),
            3 => Ok(HandshakerState::Transfer),
            _ => Err(anyhow!("HandshakerState unknown value {}", value)),
        }
    }
}

#[derive(Debug)]
struct Handshaker {
    connection: Connection,
    state: HandshakerState,
}

impl Handshaker {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            state: HandshakerState::Waiting,
        }
    }

    pub fn into_connection(self) -> Connection {
        self.connection
    }

    pub fn state(&self) -> HandshakerState {
        self.state
    }

    pub fn update(&mut self) -> Result<()> {
        if let Some((id, data)) = self.connection.recieve()? {
            let mut reader = PacketReader::new(std::io::Cursor::new(data.as_ref()));

            match id {
                0 => match self.state {
                    HandshakerState::Waiting => {
                        let handshake = packet::server_list::Handshake::packet_read(&mut reader)?;
                        self.state = handshake.next_state.try_into()?;
                    }
                    HandshakerState::Status => {
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
                            description: Some(packet::server_list::StatusResponseDescription {
                                text: "Hello, World!".to_string(),
                            }),
                            favicon: Some(format!(
                                "data:image/png;base64,{}",
                                base64::prelude::BASE64_STANDARD.encode(SERVER_ICON)
                            )),
                            enforces_secure_chat: false,
                        })?
                    }
                    HandshakerState::Login => panic!(),
                    HandshakerState::Transfer => panic!(),
                    _ => panic!(),
                },
                1 => {
                    self.connection
                        .send(packet::server_list::Ping::packet_read(&mut reader)?)?;
                    self.state = HandshakerState::Closed;
                }
                _ => panic!(),
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum LoginPlayerState {
    Login,
    Configuration,
    Play,
}

/// A player that is in the process of logging in.
#[derive(Debug)]
struct LoginPlayer {
    connection: Connection,
    last_recv_configuration_time: Instant,
    send_final_configuration_packet: bool,
    state: LoginPlayerState,
    player: Option<(String, UUID)>,
}

impl LoginPlayer {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            last_recv_configuration_time: Instant::now(),
            send_final_configuration_packet: false,
            state: LoginPlayerState::Login,
            player: None,
        }
    }

    pub fn into_connection(self) -> Connection {
        self.connection
    }

    pub fn update(&mut self) -> Result<()> {
        if let Some((id, data)) = self.connection.recieve()? {
            let mut reader = PacketReader::new(std::io::Cursor::new(data.as_ref()));

            match self.state {
                LoginPlayerState::Login => match id {
                    0 => {
                        let login_start = packet::login::LoginStart::packet_read(&mut reader)?;
                        self.player = Some((login_start.name.clone(), login_start.uuid));
                        self.connection.send(packet::login::LoginSuccess {
                            uuid: login_start.uuid,
                            name: login_start.name,
                            properties: Vec::new(),
                            strict_error_handling: false,
                        })?;
                    }
                    3 => {
                        let _login_acknowledged =
                            packet::login::LoginAcknowledged::packet_read(&mut reader)?;
                        self.last_recv_configuration_time = Instant::now();
                        self.state = LoginPlayerState::Configuration;
                        self.connection.send(
                            packet::login::LoginConfigurationClientboundKnownPacks {
                                packs: vec![packet::login::LoginConfigurationKnownPack {
                                    namespace: "minecraft:core".to_string(),
                                    id: "".to_string(),
                                    version: "1.21".to_string(),
                                }],
                            },
                        )?;
                    }
                    _ => {
                        return Err(anyhow!(
                            "Recieved unknown packet {} on LoginPlayer with Login state",
                            id
                        ))
                    }
                },
                LoginPlayerState::Configuration => {
                    self.last_recv_configuration_time = Instant::now();
                    match id {
                        0 => {
                            let _login_client_information =
                                packet::login::LoginConfigurationClientInformation::packet_read(
                                    &mut reader,
                                )?;
                        }
                        2 => {
                            let _login_configuration_plugin =
                                packet::login::LoginConfigurationPluginMessage::packet_read(
                                    &mut reader,
                                )?;
                        }
                        3 => {
                            let _login_configuration_finish_acknowledge =
                                packet::login::LoginConfigurationFinish::packet_read(&mut reader)?;
                            self.state = LoginPlayerState::Play;
                            self.connection.send(packet::login::LoginPlay {
                                entity_id: 0,
                                is_hardcore: false,
                                dimensions: vec!["minecraft:overworld".to_string()],
                                max_players: 20,
                                view_distance: 16,
                                simulation_distance: 16,
                                reduced_debug_info: false,
                                enable_respawn_screen: true,
                                do_limited_crafting: false,
                                dimension_type: 0,
                                dimension_name: "minecraft:overworld".to_owned(),
                                hashed_seed: 0,
                                game_mode: 1,
                                previous_game_mode: -1,
                                is_debug: false,
                                is_flat: false,
                                death: None,
                                portal_cooldown: 0,
                                enforces_secure_chat: false,
                            })?;
                        }
                        7 => {
                            let _login_client_known_packs = packet::login::LoginConfigurationServerboundKnownPacks::packet_read(&mut reader)?;
                            let registry_dimensions =
                                packet::login::LoginConfigurationRegistryData {
                                    registry_id: "minecraft:dimension_type".to_string(),
                                    entries: vec![
                                        packet::login::LoginConfigurationRegistryDataEntry {
                                            entry_id: "minecraft:overworld".to_string(),
                                            data: Some(NBT::Compound(
                                                vec![
                                                    ("fixed_time", NBT::Long(6000)),
                                                    ("has_skylight", NBT::Byte(1)),
                                                    ("has_ceiling", NBT::Byte(0)),
                                                    ("ultrawarm", NBT::Byte(0)),
                                                    ("natural", NBT::Byte(1)),
                                                    ("coordinate_scale", NBT::Double(1.0)),
                                                    ("bed_works", NBT::Byte(1)),
                                                    ("respawn_anchor_works", NBT::Byte(0)),
                                                    ("min_y", NBT::Int(-64)),
                                                    ("height", NBT::Int(320)),
                                                    ("logical_height", NBT::Int(256)),
                                                    ("infiniburn", NBT::String("#".to_string())),
                                                    (
                                                        "effects",
                                                        NBT::String(
                                                            "minecraft:overworld".to_string(),
                                                        ),
                                                    ),
                                                    ("ambient_light", NBT::Float(0.0)),
                                                    ("piglin_safe", NBT::Byte(0)),
                                                    ("has_raids", NBT::Byte(0)),
                                                    ("monster_spawn_light_level", NBT::Int(0)),
                                                    (
                                                        "monster_spawn_block_light_limit",
                                                        NBT::Int(0),
                                                    ),
                                                ]
                                                .into_iter()
                                                .map(|(k, v)| (k.to_string(), v))
                                                .collect::<HashMap<String, NBT>>(),
                                            )),
                                        },
                                    ],
                                };
                            self.connection.send(registry_dimensions)?;
                        }
                        _ => {
                            return Err(anyhow!(
                            "Recieved unknown packet {} on LoginPlayer with Configuration state",
                            id
                        ))
                        }
                    }
                }
                LoginPlayerState::Play => unimplemented!(),
            }
        }

        if self.state == LoginPlayerState::Configuration
            && !self.send_final_configuration_packet
            && Instant::now().duration_since(self.last_recv_configuration_time)
                > Duration::from_millis(100)
        {
            self.send_final_configuration_packet = true;
            self.connection
                .send(packet::login::LoginConfigurationFinish {})?;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Player {
    connection: Connection,
}

impl Player {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }
}

#[derive(Debug)]
struct Server {
    listener: TcpListener,
    handshakers: Vec<Handshaker>,
    login_players: Vec<LoginPlayer>,
    players: Vec<Player>,
}

impl Server {
    pub fn new<S: ToSocketAddrs>(address: S) -> Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            handshakers: Vec::new(),
            login_players: Vec::new(),
            players: Vec::new(),
        })
    }

    fn handle_handshakers(&mut self) -> Result<()> {
        while let Ok((stream, _)) = self.listener.accept() {
            let connection = Connection::new(stream)?;
            self.handshakers.push(Handshaker::new(connection));
        }

        // TODO: For each handshaker, try updating until state is either closed or login.
        self.handshakers
            .iter_mut()
            .map(|handshaker| handshaker.update())
            .collect::<Result<Vec<_>, _>>()?;

        self.handshakers
            .retain(|handshaker| handshaker.state() != HandshakerState::Closed);

        for i in (0..self.handshakers.len()).rev() {
            if self.handshakers[i].state() == HandshakerState::Login {
                let handshaker = self.handshakers.remove(i);
                let login_player = LoginPlayer::new(handshaker.into_connection());
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

        Ok(())
    }

    pub fn step(&mut self) -> Result<()> {
        self.handle_handshakers()?;
        self.handle_login_players()?;

        Ok(())
    }
}

fn main() -> Result<()> {
    let mut server = Server::new("127.0.0.1:52817")?;

    loop {
        server.step()?;
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    //Ok(())
}
