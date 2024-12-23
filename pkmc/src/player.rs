use std::collections::HashSet;

use pkmc_defs::{packet, text_component::TextComponent};
use pkmc_packet::{connection::ConnectionError, Connection};
use pkmc_util::{IterRetain as _, UUID};
use rand::Rng as _;
use thiserror::Error;

use crate::client::PlayerInformation;

#[derive(Error, Debug)]
pub enum PlayerError {
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(
        "Client bad keep alive response (No response, wrong id, or responded when not expected)"
    )]
    BadKeepAliveResponse,
}

const KEEPALIVE_PING_TIME: std::time::Duration = std::time::Duration::from_millis(10000);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct ChunkPosition {
    chunk_x: i32,
    chunk_z: i32,
}

impl ChunkPosition {
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self { chunk_x, chunk_z }
    }

    pub fn distance(&self, other: &ChunkPosition) -> f32 {
        let dx = (other.chunk_x - self.chunk_x) as f32;
        let dz = (other.chunk_z - self.chunk_z) as f32;
        (dx * dx + dz * dz).sqrt()
    }
}

// FIXME: The chunk loading radius does not match whatever the server says the view distance is.
// SEE: ChunkLoader.radius & packet::play::LoginPlay.view_distance

// FIXME: Chunk radius is biased towards negative-xz???

#[derive(Debug)]
struct ChunkLoader {
    center: Option<ChunkPosition>,
    radius: i32,
    to_load: HashSet<ChunkPosition>,
    loaded: HashSet<ChunkPosition>,
    to_unload: Vec<ChunkPosition>,
}

impl ChunkLoader {
    pub fn new(radius: i32) -> Self {
        Self {
            center: None,
            radius,
            to_load: HashSet::new(),
            loaded: HashSet::new(),
            to_unload: Vec::new(),
        }
    }

    fn iter_radius(&self) -> impl Iterator<Item = ChunkPosition> {
        let center = self.center.unwrap();
        let radius = self.radius;
        (-radius..=radius)
            .flat_map(move |dx| (-radius..=radius).map(move |dz| (dx, dz)))
            .map(move |(dx, dz)| ChunkPosition {
                chunk_x: center.chunk_x + dx,
                chunk_z: center.chunk_z + dz,
            })
            .filter(move |chunk| center.distance(chunk) < radius as f32)
    }

    /// Returns if updated center is new.
    pub fn update_center(&mut self, center: Option<ChunkPosition>) -> bool {
        if center == self.center {
            return false;
        }
        self.center = center;

        let Some(center) = center else {
            self.to_load.clear();
            self.to_unload.append(&mut self.loaded.drain().collect());
            return true;
        };

        self.to_load
            .retain(|chunk| center.distance(chunk) < self.radius as f32);
        self.to_unload.append(
            &mut self
                .loaded
                .retain_returned(|chunk| center.distance(chunk) < self.radius as f32)
                .collect(),
        );
        self.iter_radius().for_each(|chunk| {
            if self.to_load.contains(&chunk) || self.loaded.contains(&chunk) {
                return;
            }
            self.to_load.insert(chunk);
        });

        true
    }

    pub fn next_to_load(&mut self) -> Option<ChunkPosition> {
        if let Some(next) = self.to_load.iter().next().cloned() {
            self.to_load.remove(&next);
            self.loaded.insert(next);
            Some(next)
        } else {
            None
        }
    }

    pub fn next_to_unload(&mut self) -> Option<ChunkPosition> {
        self.to_unload.pop()
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct Player {
    connection: Connection,
    name: String,
    uuid: UUID,
    keepalive_time: std::time::Instant,
    keepalive_id: Option<i64>,
    x: f64,
    y: f64,
    z: f64,
    chunk_loader: ChunkLoader,
}

impl Player {
    pub fn new(
        connection: Connection,
        player_information: PlayerInformation,
    ) -> Result<Self, PlayerError> {
        let mut player = Self {
            connection,
            name: player_information.name,
            uuid: player_information.uuid,
            keepalive_time: std::time::Instant::now(),
            keepalive_id: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            chunk_loader: ChunkLoader::new(32),
        };

        player.connection.send(packet::play::Login {
            entity_id: 0,
            is_hardcore: false,
            dimensions: vec!["pkmc:void".to_owned()],
            max_players: 42069,
            view_distance: player.chunk_loader.radius,
            simulation_distance: 6,
            reduced_debug_info: false,
            enable_respawn_screen: true,
            do_limited_crafting: false,
            dimension_type: 0,
            dimension_name: "pkmc:void".to_owned(),
            hashed_seed: 0,
            game_mode: 1,
            previous_game_mode: -1,
            is_debug: false,
            is_flat: false,
            death: None,
            portal_cooldown: 0,
            sea_level: 0,
            enforces_secure_chat: false,
        })?;

        player
            .connection
            .send(packet::play::GameEvent::StartWaitingForLevelChunks)?;

        player
            .connection
            .send(packet::play::PlayerAbilities_Clientbound {
                flags: 0x01 | 0x02 | 0x04 | 0x08,
                flying_speed: 0.5,
                field_of_view_modifier: 0.1,
            })?;

        Ok(player)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> &UUID {
        &self.uuid
    }

    pub fn kick<T: Into<TextComponent>>(&mut self, text: T) -> Result<(), PlayerError> {
        self.connection
            .send(packet::play::Disconnect(text.into()))?;
        self.connection.close()?;
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.connection.is_closed()
    }

    pub fn update(&mut self) -> Result<(), PlayerError> {
        if std::time::Instant::now().duration_since(self.keepalive_time) >= KEEPALIVE_PING_TIME {
            self.keepalive_time = std::time::Instant::now();
            // Didn't respond to previous keepalive in time for new one.
            if self.keepalive_id.is_some() {
                return Err(PlayerError::BadKeepAliveResponse);
            }
            let id: i64 = rand::thread_rng().gen();
            self.keepalive_id = Some(id);
            self.connection.send(packet::play::KeepAlive { id })?;
        }

        while let Some(packet) = self.connection.recieve()? {
            match packet::play::PlayPacket::try_from(&packet)? {
                packet::play::PlayPacket::KeepAlive(keepalive) => match self.keepalive_id.take() {
                    // Success so we do nothing.
                    Some(keepalive_id) if keepalive_id == keepalive.id => {}
                    // Either responded to invalid keepalive, or keepalive id is wrong.
                    _ => return Err(PlayerError::BadKeepAliveResponse),
                },
                packet::play::PlayPacket::PlayerLoaded(_player_loaded) => {}
                packet::play::PlayPacket::MovePlayerPosRot(move_player_pos_rot) => {
                    self.x = move_player_pos_rot.x;
                    self.y = move_player_pos_rot.y;
                    self.z = move_player_pos_rot.z;
                }
                packet::play::PlayPacket::MovePlayerPos(move_player_pos) => {
                    self.x = move_player_pos.x;
                    self.y = move_player_pos.y;
                    self.z = move_player_pos.z;
                }
                packet::play::PlayPacket::MovePlayerRot(_move_player_rot) => {}
                packet::play::PlayPacket::ClientTickEnd(_client_tick_end) => {}
                packet::play::PlayPacket::PlayerInput(_player_input) => {}
                packet::play::PlayPacket::PlayerAbilities(_player_abilities) => {}
                packet::play::PlayPacket::PlayerCommand(_player_command) => {}
            }
        }

        let chunk_position = ChunkPosition::new((self.x as i32) / 16, (self.z as i32) / 16);
        if self.chunk_loader.update_center(Some(chunk_position)) {
            self.connection.send(packet::play::SetChunkCacheCenter {
                chunk_x: chunk_position.chunk_x,
                chunk_z: chunk_position.chunk_z,
            })?;
        }

        while let Some(to_unload) = self.chunk_loader.next_to_unload() {
            self.connection.send(packet::play::ForgetLevelChunk {
                chunk_x: to_unload.chunk_x,
                chunk_z: to_unload.chunk_z,
            })?;
        }

        while let Some(to_load) = self.chunk_loader.next_to_load() {
            self.connection
                .send(packet::play::LevelChunkWithLight::generate_test(
                    to_load.chunk_x,
                    to_load.chunk_z,
                    1,
                )?)?;
        }

        Ok(())
    }
}
