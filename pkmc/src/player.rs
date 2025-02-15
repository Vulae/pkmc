use std::sync::{Arc, Mutex};

use pkmc_defs::{
    biome::Biome,
    block::Block,
    entity::entity_type_id,
    packet::{
        self,
        play::{EntityAnimationType, EntityMetadataBundle},
    },
    text_component::TextComponent,
};
use pkmc_server::{
    entity_manager::{new_entity_id, Entity, EntityBase, EntityViewer},
    tab_list::{TabListPlayer, TabListViewer},
    world::{
        anvil::AnvilError,
        chunk_loader::{ChunkLoader, ChunkPosition},
        World, WorldBlock, WorldViewer,
    },
};
use pkmc_util::{
    packet::{ClientboundPacket, Connection, ConnectionError},
    IdTable, Position, Vec3, UUID,
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
struct PlayerEntity {}

impl Entity for PlayerEntity {
    fn r#type(&self) -> i32 {
        entity_type_id("minecraft:player").unwrap()
    }
}

#[derive(Debug)]
pub struct Player {
    connection: Connection,
    server_state: ServerState,
    world_viewer: Arc<Mutex<WorldViewer>>,
    entity_viewer: Option<Arc<Mutex<EntityViewer>>>,
    player_entity: Option<EntityBase<PlayerEntity>>,
    tab_list_viewer: Arc<Mutex<TabListViewer>>,
    tab_list_player: Arc<Mutex<TabListPlayer>>,
    player_name: String,
    player_uuid: UUID,
    uuid: UUID,
    keepalive_time: std::time::Instant,
    keepalive_id: Option<i64>,
    position: Vec3<f64>,
    pitch: f32,
    yaw: f32,
    is_flying: bool,
    fly_speed: f32,
    slot: u16,
}

impl Player {
    pub fn new(
        connection: Connection,
        server_state: ServerState,
        player_name: String,
        player_uuid: UUID,
        uuid: UUID,
        view_distance: u8,
    ) -> Result<Self, PlayerError> {
        let world_viewer = server_state
            .world
            .lock()
            .unwrap()
            .add_viewer(connection.sender());
        world_viewer
            .lock()
            .unwrap()
            .loader
            .update_radius(view_distance.into());

        let tab_list_viewer = server_state
            .tab_list
            .lock()
            .unwrap()
            .add_viewer(connection.sender(), uuid)?;

        let tab_list_player = server_state
            .tab_list
            .lock()
            .unwrap()
            .insert(uuid, player_name.clone());

        let mut player = Self {
            connection,
            server_state,
            world_viewer,
            entity_viewer: None,
            player_entity: None,
            tab_list_viewer,
            tab_list_player,
            player_name,
            player_uuid,
            uuid,
            keepalive_time: std::time::Instant::now(),
            keepalive_id: None,
            position: Vec3::zero(),
            pitch: 0.0,
            yaw: 0.0,
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

        player.connection.send(&packet::play::Login {
            entity_id: new_entity_id(),
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

        player.connection.send(&packet::play::ServerLinks::new([
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
            .send(&packet::play::GameEvent::StartWaitingForLevelChunks)?;

        player.connection.send(&packet::play::PlayerPosition {
            x: 0.0,
            y: 128.0,
            z: 0.0,
            ..Default::default()
        })?;

        player.update_flyspeed()?;

        player
            .connection
            .send(&packet::play::SetActionBarText(TextComponent::rainbow(
                &format!("Hello, {}!", player.player_name()),
                0.0,
            )))?;

        Ok(player)
    }

    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    pub fn player_uuid(&self) -> &UUID {
        &self.player_uuid
    }

    pub fn uuid(&self) -> &UUID {
        &self.uuid
    }

    pub fn set_view_distance(&mut self, view_distance: u8) -> Result<(), PlayerError> {
        self.world_viewer
            .lock()
            .unwrap()
            .loader
            .update_radius(view_distance.into());
        self.connection
            .send(&packet::play::SetChunkChacheRadius(view_distance as i32))?;
        Ok(())
    }

    pub fn kick<T: Into<TextComponent>>(&mut self, text: T) -> Result<(), PlayerError> {
        self.connection
            .send(&packet::play::Disconnect(text.into()))?;
        self.connection.close();
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.connection.is_closed()
    }

    fn update_flyspeed(&mut self) -> Result<(), PlayerError> {
        self.connection
            .send(&packet::play::PlayerAbilities_Clientbound {
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
            self.connection.send(&packet::play::KeepAlive { id })?;
        }

        while let Some(packet) = match self.connection.recieve_into::<packet::play::PlayPacket>() {
            Ok(packet) => packet,
            Err(err @ ConnectionError::UnsupportedPacket(..)) => {
                println!("{} {}", self.player_name(), err);
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
                packet::play::PlayPacket::PlayerLoaded(player_loaded) => {
                    let entity_viewer = self
                        .server_state
                        .entities
                        .lock()
                        .unwrap()
                        .add_viewer(self.connection.sender(), self.uuid);
                    let player_entity = self
                        .server_state
                        .entities
                        .lock()
                        .unwrap()
                        .add_entity(PlayerEntity {}, self.uuid);
                    //player_entity.handler().lock().unwrap().metadata =
                    //    EntityMetadataBundle::player_default();
                    self.entity_viewer = Some(entity_viewer);
                    self.player_entity = Some(player_entity);
                }
                packet::play::PlayPacket::AcceptTeleportation(_accept_teleportation) => {}
                packet::play::PlayPacket::MovePlayerPosRot(move_player_pos_rot) => {
                    self.position.set(
                        move_player_pos_rot.x,
                        move_player_pos_rot.y,
                        move_player_pos_rot.z,
                    );
                    self.pitch = move_player_pos_rot.pitch;
                    self.yaw = move_player_pos_rot.yaw;
                }
                packet::play::PlayPacket::MovePlayerPos(move_player_pos) => {
                    self.position
                        .set(move_player_pos.x, move_player_pos.y, move_player_pos.z);
                }
                packet::play::PlayPacket::MovePlayerRot(move_player_rot) => {
                    self.pitch = move_player_rot.pitch;
                    self.yaw = move_player_rot.yaw;
                }
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
                packet::play::PlayPacket::SwingArm(packet::play::SwingArm(is_offhand)) => {
                    if let Some(player_entity) = self.player_entity.as_ref() {
                        player_entity
                            .handler()
                            .lock()
                            .unwrap()
                            .animate(if !is_offhand {
                                EntityAnimationType::SwingMainArm
                            } else {
                                EntityAnimationType::SwingOffhand
                            })
                    }

                    let mut world = self.server_state.world.lock().unwrap();
                    if let Some(position) = Position::iter_ray(
                        self.position + Vec3::new(0.0, 1.5, 0.0),
                        Vec3::get_vector_for_rotation(self.pitch.into(), self.yaw.into()),
                        5000.0,
                    )
                    .find(|p| {
                        world
                            .get_block(*p)
                            .ok()
                            .flatten()
                            .map(|b| !b.as_block().is_air())
                            .unwrap_or(false)
                    }) {
                        Position::iter_offset(Position::iter_sphere(32.0), position).try_for_each(
                            |p| world.set_block(p, WorldBlock::Block(Block::air())),
                        )?;
                    }
                }
            }
        }

        let mut world_viewer = self.world_viewer.lock().unwrap();
        world_viewer.position = self.position;

        if let Some(player_entity) = &self.player_entity {
            let mut player_entity_handler = player_entity.handler().lock().unwrap();
            player_entity_handler.position = self.position;
            player_entity_handler.yaw = self.yaw;
            player_entity_handler.pitch = self.pitch;
            player_entity_handler.head_yaw = self.yaw;
        }

        Ok(())
    }
}
