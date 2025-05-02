#![allow(unused)]

use std::sync::{Arc, Mutex};

use pkmc_defs::{
    packet::{self, play::EntityAnimationType},
    text_component::TextComponent,
};
use pkmc_generated::{block::Block, registry::EntityType};
use pkmc_server::{
    entity_manager::{new_entity_id, Entity, EntityBase, EntityViewer},
    tab_list::{TabListPlayer, TabListViewer},
    world::{anvil::AnvilError, World, WorldViewer},
};
use pkmc_util::{
    connection::{Connection, ConnectionError, ConnectionSender},
    nbt::NBT,
    Position, Vec3, UUID,
};
use thiserror::Error;

use crate::server::{ServerState, REGISTRIES};

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
    const TYPE: EntityType = EntityType::Player;
}

#[derive(Debug)]
pub struct Player {
    connection: Connection,
    server_state: ServerState,
    world_viewer: Arc<Mutex<WorldViewer>>,
    entity_viewer: Arc<Mutex<EntityViewer>>,
    player_entity: EntityBase<PlayerEntity>,
    _tab_list_viewer: Arc<Mutex<TabListViewer>>,
    _tab_list_player: Arc<Mutex<TabListPlayer>>,
    _server_tab_list_info_viewer: Arc<Mutex<ConnectionSender>>,
    player_name: String,
    _player_uuid: UUID,
    _uuid: UUID,
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
        entity_distance: f64,
    ) -> Result<Self, PlayerError> {
        let dimension = server_state.world.lock().unwrap().identifier().to_owned();

        connection.send(&packet::play::Login {
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
            game_mode: packet::play::Gamemode::Creative,
            previous_game_mode: None,
            is_debug: false,
            is_flat: false,
            death: None,
            portal_cooldown: 0,
            sea_level: 0,
            enforces_secure_chat: false,
        })?;

        connection.send(&packet::play::ServerLinks::new([
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

        connection.send(&packet::play::GameEvent::StartWaitingForLevelChunks)?;

        connection.send(&packet::play::PlayerPosition {
            x: 0.0,
            y: 128.0,
            z: 0.0,
            ..Default::default()
        })?;

        connection.send(&packet::play::SystemChat {
            content: TextComponent::rainbow(&format!("Hello, {}!", player_name), 0.0),
            overlay: false,
        })?;

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

        let entity_viewer = server_state
            .entities
            .lock()
            .unwrap()
            .add_viewer(connection.sender(), uuid);
        entity_viewer.lock().unwrap().radius = entity_distance;
        let player_entity = server_state
            .entities
            .lock()
            .unwrap()
            .add_entity(PlayerEntity {}, uuid);
        player_entity
            .handler()
            .lock()
            .unwrap()
            .visibility
            .exclude(uuid);

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

        let server_tab_list_info_viewer = server_state
            .server_tab_info
            .lock()
            .unwrap()
            .add_viewer(connection.sender());

        let mut player = Self {
            connection,
            server_state,
            world_viewer,
            entity_viewer,
            player_entity,
            _tab_list_viewer: tab_list_viewer,
            _tab_list_player: tab_list_player,
            _server_tab_list_info_viewer: server_tab_list_info_viewer,
            player_name,
            _player_uuid: player_uuid,
            _uuid: uuid,
            keepalive_time: std::time::Instant::now(),
            keepalive_id: None,
            position: Vec3::zero(),
            pitch: 0.0,
            yaw: 0.0,
            is_flying: true,
            fly_speed: 0.1,
            slot: 0,
        };

        player.update_flyspeed()?;

        Ok(player)
    }

    pub fn player_name(&self) -> &str {
        &self.player_name
    }

    #[allow(unused)]
    pub fn player_uuid(&self) -> &UUID {
        &self._player_uuid
    }

    #[allow(unused)]
    pub fn uuid(&self) -> &UUID {
        &self._uuid
    }

    #[allow(unused)]
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

    #[allow(unused)]
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

    fn resend_block(
        &mut self,
        location: Position,
        sequence: Option<i32>,
    ) -> Result<(), PlayerError> {
        let mut world = self.server_state.world.lock().unwrap();
        if let Some(block) = world.get_block(location)? {
            self.connection
                .send(&packet::play::BlockUpdate { location, block })?;
            if let Some(sequence) = sequence {
                self.connection
                    .send(&packet::play::AcknowledgeBlockChange(sequence))?;
            }
            if let Some(data) = world.query_block_data(location)? {
                self.connection.send(&packet::play::BlockEntityData {
                    location,
                    r#type: data.r#type(),
                    data: NBT::Compound(data.data.clone()),
                })?;
            }
        }
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), PlayerError> {
        if std::time::Instant::now().duration_since(self.keepalive_time) >= KEEPALIVE_PING_TIME {
            self.keepalive_time = std::time::Instant::now();
            // Didn't respond to previous keepalive in time for new one.
            if self.keepalive_id.is_some() {
                return Err(PlayerError::BadKeepAliveResponse);
            }
            let id: i64 = rand::random();
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
                packet::play::PlayPacket::Ping(ping) => self.connection.send(&ping)?,
                packet::play::PlayPacket::PlayerLoaded(_player_loaded) => {
                    println!("Player {} loaded!", self.player_name());
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
                    self.is_flying = player_abilities.flags & 0x02 != 0;
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
                packet::play::PlayPacket::SwingArm(packet::play::SwingArm(hand)) => {
                    self.player_entity
                        .handler()
                        .lock()
                        .unwrap()
                        .animate(match hand {
                            packet::play::Hand::Mainhand => EntityAnimationType::SwingMainArm,
                            packet::play::Hand::Offhand => EntityAnimationType::SwingOffhand,
                        });
                }
                packet::play::PlayPacket::UseItemOn(use_item_on) => {
                    {
                        let mut world = self.server_state.world.lock().unwrap();
                        if let Some(data) = world.query_block_data(use_item_on.location)? {
                            println!("{:#?}", data);
                            self.connection.send(&packet::play::SystemChat {
                                content: TextComponent::new(format!("{:#?}", data)),
                                overlay: false,
                            })?;
                        }
                    }
                    self.resend_block(use_item_on.location, Some(use_item_on.sequence))?;
                }
                packet::play::PlayPacket::PlayerAction(player_action) => {
                    match player_action.status {
                        packet::play::PlayerActionStatus::FinishedDigging
                        | packet::play::PlayerActionStatus::StartedDigging => {
                            self.resend_block(player_action.location, Some(player_action.sequence));
                        }
                        packet::play::PlayerActionStatus::SwapItemInHand => {
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
                                    .map(|b| !b.is_air())
                                    .unwrap_or(false)
                            }) {
                                Position::iter_offset(Position::iter_sphere(32.0), position)
                                    .try_for_each(|p| world.set_block(p, Block::Air))?;
                            }
                        }
                        _ => {}
                    }
                }
                packet::play::PlayPacket::ChatMessage(chat_message) => {
                    self.connection.send(&packet::play::DisguisedChatMessage {
                        message: TextComponent::from(chat_message.message),
                        chat_type: 0,
                        sender_name: TextComponent::from(self.player_name()),
                        target_name: None,
                    })?;
                }
            }
        }

        self.world_viewer.lock().unwrap().position = self.position;
        self.entity_viewer.lock().unwrap().position = self.position;

        let mut player_entity_handler = self.player_entity.handler().lock().unwrap();
        player_entity_handler.position = self.position;
        player_entity_handler.yaw = self.yaw;
        player_entity_handler.pitch = self.pitch;
        player_entity_handler.head_yaw = self.yaw;

        Ok(())
    }
}
