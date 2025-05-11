#![allow(unused)]

use std::sync::{Arc, Mutex, MutexGuard};

use pkmc_defs::{
    dimension::Dimension,
    packet::{
        self,
        play::{CommandNode, CommandNodeParser, EntityAnimationType},
    },
    text_component::TextComponent,
};
use pkmc_generated::{block::Block, registry::EntityType};
use pkmc_server::{
    command::{
        CommandCoordinateTransform, CommandListener, CommandManager, CommandParsableCoordinate,
        CommandParseError, CommandParser,
    },
    entity_manager::{new_entity_id, Entity, EntityBase, EntityViewer},
    level::{
        anvil::{AnvilError, AnvilLevel, AnvilWorld},
        Level, LevelViewer,
    },
    tab_list::{TabListPlayer, TabListViewer},
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
    #[error("Could not find dimension {0:?}")]
    CouldNotFindDimension(Dimension),
}

#[derive(Debug)]
struct PlayerEntity {}

impl Entity for PlayerEntity {
    fn r#type(&self) -> EntityType {
        EntityType::Player
    }
}

pub trait PlayerExecutableCommand: CommandListener {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError>;
}

#[derive(Debug)]
struct CommandGoto {
    position: Vec3<f64>,
    dimension: Option<String>,
}

impl CommandListener for CommandGoto {
    fn node() -> CommandNode {
        CommandNode::new_literal("goto").with_child(
            CommandNode::new(CommandNodeParser::Vec3).with_child(
                CommandNode::new_literal("in").with_child(CommandNode::new(
                    CommandNodeParser::ResourceKey("minecraft:dimension_type".to_owned()),
                )),
            ),
        )
    }

    type ParseArg = CommandCoordinateTransform;
    fn try_parse(
        parser: &mut CommandParser<'_>,
        arg: &Self::ParseArg,
    ) -> Result<Option<Self>, CommandParseError> {
        if parser.consume_literal("goto").is_err() {
            return Ok(None);
        }
        Ok(Some(Self {
            position: parser
                .consume::<CommandParsableCoordinate>()?
                .to_coordinates(arg),
            dimension: if parser.consume_until_space_or_end(false)? == "in" {
                Some(parser.consume_until_space_or_end(true)?.to_owned())
            } else {
                None
            },
        }))
    }
}

impl PlayerExecutableCommand for CommandGoto {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError> {
        if let Some(dimension) = self.dimension {
            match player.set_dimension(dimension.into(), self.position) {
                Err(PlayerError::CouldNotFindDimension(dimension)) => {
                    player.system_message(format!(
                        "Could not find dimension \"{:?}\"",
                        dimension.name()
                    ))?;
                }
                res => res?,
            }
        } else {
            player.teleport(self.position)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct CommandData {
    position: Position,
}

impl CommandListener for CommandData {
    fn node() -> CommandNode {
        CommandNode::new_literal("data")
            .with_child(CommandNode::new(CommandNodeParser::BlockPosition))
    }

    type ParseArg = CommandCoordinateTransform;
    fn try_parse(
        parser: &mut CommandParser<'_>,
        arg: &Self::ParseArg,
    ) -> Result<Option<Self>, CommandParseError> {
        if parser.consume_literal("data").is_err() {
            return Ok(None);
        }
        Ok(Some(Self {
            position: parser
                .consume::<CommandParsableCoordinate>()?
                .to_position(arg)?,
        }))
    }
}

impl PlayerExecutableCommand for CommandData {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError> {
        let mut level_mutex = player.level.clone();
        let mut level = level_mutex.lock().unwrap();
        if let Some(data) = level.query_block_data(self.position)? {
            player.system_message(format!("Block data at {}: {:#?}", self.position, data.data))?;
        } else {
            player.system_message(format!("No block data at {}", self.position))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
enum CommandDestroy {
    Sphere { radius: f32 },
    Cube { pos1: Position, pos2: Position },
}

impl CommandListener for CommandDestroy {
    fn node() -> CommandNode {
        CommandNode::new_literal("destroy")
            .with_child(
                CommandNode::new_literal("sphere").with_child(CommandNode::new(
                    CommandNodeParser::Float {
                        min: None,
                        max: Some(256.0),
                    },
                )),
            )
            .with_child(
                CommandNode::new_literal("cube").with_child(
                    CommandNode::new(CommandNodeParser::BlockPosition)
                        .with_child(CommandNode::new(CommandNodeParser::BlockPosition)),
                ),
            )
    }

    type ParseArg = CommandCoordinateTransform;
    fn try_parse(
        parser: &mut CommandParser<'_>,
        arg: &Self::ParseArg,
    ) -> Result<Option<Self>, CommandParseError> {
        if parser.consume_literal("destroy").is_err() {
            return Ok(None);
        }
        match parser.consume_until_space_or_end(true)? {
            "sphere" => Ok(Some(Self::Sphere {
                radius: parser.consume_until_space_or_end(true)?.parse()?,
            })),
            "cube" => Ok(Some(Self::Cube {
                pos1: parser
                    .consume::<CommandParsableCoordinate>()?
                    .to_position(arg)?,
                pos2: parser
                    .consume::<CommandParsableCoordinate>()?
                    .to_position(arg)?,
            })),
            _ => Err(CommandParseError::Custom(
                "Expected valid literal argument".to_owned(),
            )),
        }
    }
}

impl PlayerExecutableCommand for CommandDestroy {
    fn execute(self, player: &mut Player) -> Result<(), PlayerError> {
        let mut level = player.level.lock().unwrap();
        match self {
            CommandDestroy::Sphere { radius } => {
                if let Some(position) = Position::iter_ray(
                    player.position + Vec3::new(0.0, 1.5, 0.0),
                    Vec3::get_vector_for_rotation(player.pitch.into(), player.yaw.into()),
                    5000.0,
                )
                .find(|p| {
                    level
                        .get_block(*p)
                        .ok()
                        .flatten()
                        .map(|b| !b.is_air())
                        .unwrap_or(false)
                }) {
                    Position::iter_offset(Position::iter_sphere(radius), position)
                        .try_for_each(|p| level.set_block(p, Block::Air))?;
                }
            }
            CommandDestroy::Cube { pos1, pos2 } => {
                let (min, max) = Position::fix_boundaries(pos1, pos2);
                let size = max - min;
                Position::iter_offset(Position::iter_cube(size.x + 1, size.y + 1, size.z + 1), min)
                    .try_for_each(|p| level.set_block(p, Block::Air))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Player {
    connection: Connection,
    server_state: ServerState,
    view_distance: u8,
    level: Arc<Mutex<AnvilLevel>>,
    world_viewer: Arc<Mutex<LevelViewer>>,
    entity_viewer: Arc<Mutex<EntityViewer>>,
    player_entity: EntityBase<PlayerEntity>,
    _tab_list_viewer: Arc<Mutex<TabListViewer>>,
    _tab_list_player: Arc<Mutex<TabListPlayer>>,
    _server_tab_list_info_viewer: Arc<Mutex<ConnectionSender>>,
    command_manager: CommandManager,
    name: String,
    uuid: UUID,
    keepalive_time: std::time::Instant,
    keepalive_id: Option<i64>,
    entity_id: i32,
    position: Vec3<f64>,
    pitch: f32,
    yaw: f32,
    is_flying: bool,
    fly_speed: f32,
    slot: u16,
}

impl Player {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        connection: Connection,
        server_state: ServerState,
        name: String,
        uuid: UUID,
        player_properties: Vec<packet::login::FinishedProperty>,
        player_info: packet::configuration::ClientInformation,
        view_distance: u8,
        entity_distance: f64,
        dimension: Dimension,
    ) -> Result<Self, PlayerError> {
        let entity_id = new_entity_id();

        let level = server_state
            .world
            .lock()
            .unwrap()
            .level(&dimension)
            .ok_or(PlayerError::CouldNotFindDimension(dimension.clone()))?;

        connection.send(&packet::play::Login {
            entity_id,
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
                .find(|(_, v)| *v == dimension.name())
                .unwrap()
                .0 as i32,
            dimension_name: dimension.name().to_owned(),
            hashed_seed: 0,
            game_mode: packet::play::Gamemode::Survival,
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
            content: TextComponent::rainbow(&format!("Hello, {}!", name), 0.0),
            overlay: false,
        })?;

        let world_viewer = level.lock().unwrap().add_viewer(connection.sender());
        world_viewer
            .lock()
            .unwrap()
            .loader
            .update_radius(view_distance.into());

        let entity_viewer = server_state.entities.lock().unwrap().add_viewer(
            connection.sender(),
            uuid,
            Some(entity_id),
        );
        entity_viewer.lock().unwrap().radius = entity_distance;
        let player_entity = server_state.entities.lock().unwrap().add_entity_with_id(
            PlayerEntity {},
            uuid,
            entity_id,
        );
        {
            let mut player_entity_handler = player_entity.handler().lock().unwrap();
            player_entity_handler
                .metadata
                .player_skin_parts(player_info.displayed_skin_parts);
        }

        let tab_list_viewer = server_state
            .tab_list
            .lock()
            .unwrap()
            .add_viewer(connection.sender(), uuid)?;

        let tab_list_player = server_state
            .tab_list
            .lock()
            .unwrap()
            .insert(TabListPlayer::new(
                uuid,
                name.clone(),
                player_properties
                    .into_iter()
                    .map(|v| {
                        (
                            v.name,
                            packet::play::PlayerInfoPlayerProperty {
                                value: v.value,
                                signature: v.signature,
                            },
                        )
                    })
                    .collect(),
            ));

        let server_tab_list_info_viewer = server_state
            .server_tab_info
            .lock()
            .unwrap()
            .add_viewer(connection.sender());

        let mut player = Self {
            connection,
            server_state,
            view_distance,
            level,
            world_viewer,
            entity_viewer,
            player_entity,
            _tab_list_viewer: tab_list_viewer,
            _tab_list_player: tab_list_player,
            _server_tab_list_info_viewer: server_tab_list_info_viewer,
            command_manager: CommandManager::default(),
            name,
            uuid,
            keepalive_time: std::time::Instant::now(),
            keepalive_id: None,
            entity_id,
            position: Vec3::zero(),
            pitch: 0.0,
            yaw: 0.0,
            is_flying: true,
            fly_speed: 0.1,
            slot: 0,
        };

        player.define_commands()?;
        player.update_flyspeed()?;

        Ok(player)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> &UUID {
        &self.uuid
    }

    pub fn coordinate_transform(&self) -> CommandCoordinateTransform {
        CommandCoordinateTransform::new_partial(
            self.position,
            self.position + Vec3::new(0.0, 1.5, 0.0),
            Vec3::get_vector_for_rotation(self.pitch.into(), self.yaw.into()),
        )
    }

    pub fn execute_command<C: PlayerExecutableCommand>(
        &mut self,
        command: C,
    ) -> Result<(), PlayerError> {
        command.execute(self)
    }

    fn try_parse_then_execute_command<C: PlayerExecutableCommand>(
        &mut self,
        packet: &packet::play::ChatCommand,
        arg: &C::ParseArg,
    ) -> Result<(), PlayerError> {
        self.command_manager
            .try_parse::<C>(packet, arg, &self.connection.sender())?
            .map(|c| c.execute(self))
            .transpose()?;
        Ok(())
    }

    pub fn define_commands(&mut self) -> Result<(), PlayerError> {
        self.command_manager.register::<CommandGoto>();
        self.command_manager.register::<CommandData>();
        self.command_manager.register::<CommandDestroy>();
        self.command_manager
            .update_client_command_list(&self.connection.sender())?;
        Ok(())
    }

    pub fn parse_then_execute_command(
        &mut self,
        packet: &packet::play::ChatCommand,
    ) -> Result<(), PlayerError> {
        let transform = self.coordinate_transform();
        self.try_parse_then_execute_command::<CommandGoto>(packet, &transform)?;
        self.try_parse_then_execute_command::<CommandData>(packet, &transform)?;
        self.try_parse_then_execute_command::<CommandDestroy>(packet, &transform)?;
        Ok(())
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

    fn system_message<T: Into<TextComponent>>(&mut self, message: T) -> Result<(), PlayerError> {
        self.connection.send(&packet::play::SystemChat {
            content: message.into(),
            overlay: false,
        })?;
        Ok(())
    }

    fn teleport(&mut self, position: Vec3<f64>) -> Result<(), PlayerError> {
        self.position = position;
        self.connection.send(&packet::play::PlayerPosition {
            x: position.x,
            y: position.y,
            z: position.z,
            angle_relative: true,
            ..Default::default()
        })?;
        Ok(())
    }

    fn set_dimension(
        &mut self,
        dimension: Dimension,
        position: Vec3<f64>,
    ) -> Result<(), PlayerError> {
        self.level = self
            .server_state
            .world
            .lock()
            .unwrap()
            .level(&dimension)
            .ok_or(PlayerError::CouldNotFindDimension(dimension.clone()))?;
        self.connection.send(&packet::play::Respawn {
            dimension_type: REGISTRIES
                .get("minecraft:dimension_type")
                .unwrap()
                .keys()
                .enumerate()
                .find(|(_, v)| *v == dimension.name())
                .unwrap()
                .0 as i32,
            dimension_name: dimension.name().to_owned(),
            hashed_seed: 0,
            game_mode: packet::play::Gamemode::Survival,
            previous_game_mode: None,
            is_debug: false,
            is_flat: false,
            death: None,
            portal_cooldown: 0,
            sea_level: 0,
            data_kept: 0x03,
        })?;
        self.connection
            .send(&packet::play::GameEvent::StartWaitingForLevelChunks)?;
        self.world_viewer = self
            .level
            .lock()
            .unwrap()
            .add_viewer(self.connection.sender());
        self.world_viewer
            .lock()
            .unwrap()
            .loader
            .update_radius(self.view_distance.into());
        self.teleport(position)?;
        self.update_flyspeed()?;
        Ok(())
    }

    fn resend_block(
        &mut self,
        location: Position,
        sequence: Option<i32>,
    ) -> Result<(), PlayerError> {
        let mut level = self.level.lock().unwrap();
        if let Some(block) = level.get_block(location)? {
            self.connection
                .send(&packet::play::BlockUpdate { location, block })?;
            if let Some(sequence) = sequence {
                self.connection
                    .send(&packet::play::AcknowledgeBlockChange(sequence))?;
            }
            if let Some(data) = level.query_block_data(location)? {
                self.connection.send(&packet::play::BlockEntityData {
                    location,
                    r#type: data.r#type(),
                    data: &data.data,
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
                packet::play::PlayPacket::Ping(ping) => self.connection.send(&ping)?,
                packet::play::PlayPacket::PlayerLoaded(_player_loaded) => {}
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
                    self.resend_block(use_item_on.location, Some(use_item_on.sequence))?;
                }
                packet::play::PlayPacket::PlayerAction(player_action) => {
                    match player_action.status {
                        packet::play::PlayerActionStatus::FinishedDigging
                        | packet::play::PlayerActionStatus::StartedDigging => {
                            self.resend_block(player_action.location, Some(player_action.sequence));
                        }
                        packet::play::PlayerActionStatus::SwapItemInHand => {
                            self.execute_command(CommandDestroy::Sphere { radius: 32.0 })?;
                        }
                        _ => {}
                    }
                }
                packet::play::PlayPacket::ChatMessage(chat_message) => {
                    self.connection.send(&packet::play::DisguisedChatMessage {
                        message: TextComponent::rainbow(&chat_message.message, 0.0),
                        chat_type: 0,
                        sender_name: TextComponent::from(self.name()),
                        target_name: None,
                    })?;
                }
                packet::play::PlayPacket::ChatCommand(command) => {
                    self.parse_then_execute_command(&command)?;
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
