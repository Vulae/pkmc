use std::sync::{Arc, Mutex};

use pkmc_defs::{biome::Biome, packet, text_component::TextComponent};
use pkmc_server::world::{
    anvil::AnvilError,
    chunk_loader::{ChunkLoader, ChunkPosition},
    World, WorldViewer,
};
use pkmc_util::{
    packet::{ClientboundPacket, Connection, ConnectionError},
    IdTable, UUID,
};
use rand::Rng as _;
use thiserror::Error;

use crate::{ServerState, REGISTRIES};

const KEEPALIVE_PING_TIME: std::time::Duration = std::time::Duration::from_millis(10000);

#[derive(Error, Debug)]
pub enum PlayerError {
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    WorldError(#[from] AnvilError),
    #[error(
        "Client bad keep alive response (No response, wrong id, or responded when not expected)"
    )]
    BadKeepAliveResponse,
}

#[derive(Debug)]
pub struct Player {
    connection: Connection,
    server_state: ServerState,
    viewer: Arc<Mutex<WorldViewer>>,
    name: String,
    uuid: UUID,
    keepalive_time: std::time::Instant,
    keepalive_id: Option<i64>,
    x: f64,
    y: f64,
    z: f64,
    is_flying: bool,
    fly_speed: f32,
    slot: u16,
}

impl Player {
    pub fn new(
        connection: Connection,
        server_state: ServerState,
        uuid: UUID,
        name: String,
        view_distance: u8,
    ) -> Result<Self, PlayerError> {
        let viewer = server_state
            .world
            .lock()
            .unwrap()
            .add_viewer(connection.sender());

        viewer
            .lock()
            .unwrap()
            .loader
            .update_radius(view_distance.into());

        let mut player = Self {
            connection,
            server_state,
            viewer,
            name,
            uuid,
            keepalive_time: std::time::Instant::now(),
            keepalive_id: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            is_flying: true,
            fly_speed: 0.1,
            slot: 0,
        };

        let dimension = player
            .server_state
            .world
            .lock()
            .unwrap()
            .identifier()
            .to_owned();

        player.connection.send(packet::play::Login {
            entity_id: 0,
            is_hardcore: false,
            dimensions: REGISTRIES
                .get("minecraft:dimension_type")
                .unwrap()
                .keys()
                .cloned()
                .collect(),
            max_players: 42069,
            view_distance: view_distance as i32,
            simulation_distance: 6,
            reduced_debug_info: false,
            enable_respawn_screen: true,
            do_limited_crafting: false,
            dimension_type: REGISTRIES
                .get("minecraft:dimension_type")
                .unwrap()
                .keys()
                .enumerate()
                .find(|(_, v)| *v == &dimension)
                .unwrap()
                .0 as i32,
            dimension_name: dimension,
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

        player.connection.send(packet::play::ServerLinks::new([
            (
                packet::play::ServerLink::Website,
                "https://github.com/Vulae/pkmc",
            ),
            (
                packet::play::ServerLink::BugReport,
                "https://github.com/Vulae/pkmc/issues",
            ),
            (
                packet::play::ServerLink::Feedback,
                "https://github.com/Vulae/pkmc/issues",
            ),
        ]))?;

        player
            .connection
            .send(packet::play::GameEvent::StartWaitingForLevelChunks)?;

        player.connection.send(packet::play::PlayerPosition {
            x: 0.0,
            y: 128.0,
            z: 0.0,
            ..Default::default()
        })?;

        player.update_flyspeed()?;

        player
            .connection
            .send(packet::play::SetActionBarText(TextComponent::rainbow(
                &format!("Hello, {}!", player.name()),
                0.0,
            )))?;

        Ok(player)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> &UUID {
        &self.uuid
    }

    pub fn set_view_distance(&mut self, view_distance: u8) -> Result<(), PlayerError> {
        self.viewer
            .lock()
            .unwrap()
            .loader
            .update_radius(view_distance.into());
        self.connection
            .send(packet::play::SetChunkChacheRadius(view_distance as i32))?;
        Ok(())
    }

    pub fn kick<T: Into<TextComponent>>(&mut self, text: T) -> Result<(), PlayerError> {
        self.connection
            .send(packet::play::Disconnect(text.into()))?;
        self.connection.close();
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.connection.is_closed()
    }

    fn update_flyspeed(&mut self) -> Result<(), PlayerError> {
        self.connection
            .send(packet::play::PlayerAbilities_Clientbound {
                flags: 0x01 | if self.is_flying { 0x02 } else { 0 } | 0x04,
                flying_speed: self.fly_speed,
                field_of_view_modifier: 0.1,
            })?;
        Ok(())
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

        while let Some(packet) = match self.connection.recieve_into::<packet::play::PlayPacket>() {
            Ok(packet) => packet,
            Err(err @ ConnectionError::UnsupportedPacket(..)) => {
                println!("{} {}", self.name(), err);
                None
            }
            Err(err) => Err(err)?,
        } {
            match packet {
                packet::play::PlayPacket::KeepAlive(keepalive) => match self.keepalive_id.take() {
                    // Success so we do nothing.
                    Some(keepalive_id) if keepalive_id == keepalive.id => {}
                    // Either responded to invalid keepalive, or keepalive id is wrong.
                    _ => return Err(PlayerError::BadKeepAliveResponse),
                },
                packet::play::PlayPacket::PlayerLoaded(_player_loaded) => {}
                packet::play::PlayPacket::AcceptTeleportation(_accept_teleportation) => {}
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
                packet::play::PlayPacket::MovePlayerStatusOnly(_move_player_status_only) => {}
                packet::play::PlayPacket::ClientTickEnd(_client_tick_end) => {}
                packet::play::PlayPacket::PlayerInput(_player_input) => {}
                packet::play::PlayPacket::PlayerAbilities(player_abilities) => {
                    self.is_flying = (player_abilities.flags & 0x02 != 0);
                }
                packet::play::PlayPacket::PlayerCommand(_player_command) => {}
                packet::play::PlayPacket::SetHeldItem(set_held_item) => {
                    let new_slot = set_held_item.0;
                    let mut distance = new_slot as i16 - self.slot as i16;
                    if distance.abs() > 5 {
                        distance = if distance > 0 {
                            distance - 9
                        } else {
                            distance + 9
                        }
                    }
                    match distance {
                        0 => {}
                        ..0 => self.fly_speed *= 1.2,
                        1.. => self.fly_speed /= 1.2,
                    }
                    self.update_flyspeed()?;
                    self.slot = new_slot;
                }
            }
        }

        let mut viewer = self.viewer.lock().unwrap();
        viewer.x = self.x;
        viewer.y = self.y;
        viewer.z = self.z;

        Ok(())
    }
}
