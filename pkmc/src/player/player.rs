use std::{
    io::Write,
    sync::{Arc, RwLock},
};

use pkmc_defs::{
    generated::{
        generated, PALETTED_DATA_BIOMES_DIRECT, PALETTED_DATA_BIOMES_INDIRECT,
        PALETTED_DATA_BLOCKS_DIRECT, PALETTED_DATA_BLOCKS_INDIRECT,
    },
    packet,
    text_component::TextComponent,
    REGISTRY,
};
use pkmc_nbt::nbt_compound;
use pkmc_packet::{to_paletted_data, BitSet, Connection};
use pkmc_util::UUID;
use rand::Rng as _;

use crate::{client::PlayerInformation, server_state::ServerState};

use super::{ChunkLoader, ChunkPosition, PlayerError};

const KEEPALIVE_PING_TIME: std::time::Duration = std::time::Duration::from_millis(10000);

// NOTE: Temporary stuff for testing
const PLAYER_DIMENSION: &str = "minecraft:overworld";
const PLAYER_DIMENSION_SECTIONS: usize = 24;
const VIEW_DISTANCE: i32 = 32;

#[derive(Debug)]
pub struct Player {
    connection: Connection,
    server_state: Arc<RwLock<ServerState>>,
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
        server_state: Arc<RwLock<ServerState>>,
        player_information: PlayerInformation,
    ) -> Result<Self, PlayerError> {
        let mut player = Self {
            connection,
            server_state,
            name: player_information.name,
            uuid: player_information.uuid,
            keepalive_time: std::time::Instant::now(),
            keepalive_id: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            chunk_loader: ChunkLoader::new(VIEW_DISTANCE),
        };

        player.connection.send(packet::play::Login {
            entity_id: 0,
            is_hardcore: false,
            dimensions: REGISTRY
                .get("minecraft:dimension_type")
                .unwrap()
                .keys()
                .cloned()
                .collect(),
            max_players: 42069,
            view_distance: player.chunk_loader.radius,
            simulation_distance: 6,
            reduced_debug_info: false,
            enable_respawn_screen: true,
            do_limited_crafting: false,
            dimension_type: REGISTRY
                .get("minecraft:dimension_type")
                .unwrap()
                .keys()
                .enumerate()
                .find(|(_, v)| *v == PLAYER_DIMENSION)
                .unwrap()
                .0 as i32,
            dimension_name: PLAYER_DIMENSION.to_owned(),
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

        player.connection.send(packet::play::PlayerPosition {
            x: 0.0,
            y: 128.0,
            z: 0.0,
            ..Default::default()
        })?;

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

        while let Some(packet) = match self.connection.recieve_into::<packet::play::PlayPacket>() {
            Ok(packet) => packet,
            Err(err @ pkmc_packet::ConnectionError::UnsupportedPacket(..)) => {
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

        let server_state = self.server_state.read().unwrap();
        let mut world = server_state.world.lock().unwrap();
        let level = world.get_level("minecraft:overworld").unwrap();
        // TODO: Instead of loading only 1 chunk per update, load many until a certain time limit
        // threshold is reached.
        if let Some(to_load) = self.chunk_loader.next_to_load() {
            //println!("Load chunk: {} {}", to_load.chunk_x, to_load.chunk_z);
            if let Some(chunk) = level.get_chunk(to_load.chunk_x, to_load.chunk_z)? {
                self.connection.send(packet::play::LevelChunkWithLight {
                    chunk_x: to_load.chunk_x,
                    chunk_z: to_load.chunk_z,
                    heightmaps: nbt_compound!(),
                    data: {
                        // NOTE: Slow load data (Load data, parse it, then write to packet)
                        let mut writer = Vec::new();

                        chunk.iter_sections().try_for_each(|section| {
                            let block_ids = section.blocks_ids().unwrap();
                            // Num non-air blocks
                            let block_count = block_ids
                                .iter()
                                .filter(|b| !generated::block::is_air(**b))
                                .count();
                            writer.write_all(&(block_count as u16).to_be_bytes())?;
                            // Blocks
                            writer.write_all(&to_paletted_data(
                                &block_ids,
                                PALETTED_DATA_BLOCKS_INDIRECT,
                                PALETTED_DATA_BLOCKS_DIRECT,
                            )?)?;
                            // Biome
                            writer.write_all(&to_paletted_data(
                                &[0; 64],
                                PALETTED_DATA_BIOMES_INDIRECT,
                                PALETTED_DATA_BIOMES_DIRECT,
                            )?)?;
                            Ok::<_, PlayerError>(())
                        })?;

                        writer.into_boxed_slice()
                    },
                    block_entities: Vec::new(),
                    sky_light_mask: BitSet::new(PLAYER_DIMENSION_SECTIONS + 2),
                    block_light_mask: BitSet::new(PLAYER_DIMENSION_SECTIONS + 2),
                    empty_sky_light_mask: BitSet::new(PLAYER_DIMENSION_SECTIONS + 2),
                    empty_block_light_mask: BitSet::new(PLAYER_DIMENSION_SECTIONS + 2),
                    sky_lights_arrays: Vec::new(),
                    block_lights_arrays: Vec::new(),
                })?;
            } else {
                self.connection
                    .send(packet::play::LevelChunkWithLight::generate_test(
                        to_load.chunk_x,
                        to_load.chunk_z,
                        PLAYER_DIMENSION_SECTIONS,
                    )?)?;
            }
        }

        Ok(())
    }
}
