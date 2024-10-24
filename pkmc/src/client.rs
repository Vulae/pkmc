use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use pkmc_defs::packet;
use pkmc_nbt::nbt_compound;
use pkmc_packet::{
    create_packet_enum, to_paletted_container, BitSet, Connection, PacketWriter, Paletteable,
};
use pkmc_util::{VecExt as _, UUID};
use rand::{thread_rng, Rng};

use crate::{client_login::ClientLogin, server_state::ServerState};

create_packet_enum!(ClientPlayPacket;
    packet::play::KeepAlive, KeepAlive;
    packet::play::ConfirmTeleport, ConfirmTeleport;
    packet::play::SetPlayerPositionAndRotation, SetPlayerPositionAndRotation;
    packet::play::SetPlayerPosition, SetPlayerPosition;
    packet::play::SetPlayerRotation, SetPlayerRotation;
);

const KEEPALIVE_PING_TIME: Duration = Duration::from_millis(10000);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct ChunkPosition {
    chunk_x: i32,
    chunk_z: i32,
}

#[derive(Debug)]
struct ChunkLoader {
    center: Option<ChunkPosition>,
    radius: f64,
    to_load: Vec<ChunkPosition>,
    loaded: Vec<ChunkPosition>,
    to_unload: Vec<ChunkPosition>,
}

impl ChunkLoader {
    pub fn new(radius: f64) -> Self {
        Self {
            center: None,
            radius,
            to_load: Vec::new(),
            loaded: Vec::new(),
            to_unload: Vec::new(),
        }
    }

    /// Returns if updated center is new.
    pub fn update_center(&mut self, center: ChunkPosition) -> bool {
        if let Some(old_center) = self.center {
            if old_center == center {
                return false;
            }
        }

        let contains = |position: &ChunkPosition| -> bool {
            (((center.chunk_x - position.chunk_x).pow(2)
                + (center.chunk_z - position.chunk_z).pow(2)) as f64)
                .sqrt()
                <= self.radius
        };

        // Unload outside chunks
        self.to_unload
            .append(&mut self.to_load.retain_returned(contains));
        self.to_unload
            .append(&mut self.loaded.retain_returned(contains));

        // Load new chunks in radius
        for dx in -(self.radius.floor() as i32)..=(self.radius.ceil() as i32) {
            for dz in -(self.radius.floor() as i32)..=(self.radius.ceil() as i32) {
                let position = ChunkPosition {
                    chunk_x: center.chunk_x + dx,
                    chunk_z: center.chunk_z + dz,
                };
                if !contains(&position) {
                    continue;
                }
                if self.to_load.contains(&position) || self.loaded.contains(&position) {
                    continue;
                }
                self.to_load.push(position);
            }
        }

        self.center = Some(center);

        true
    }

    pub fn next_to_load(&mut self) -> Option<ChunkPosition> {
        if let Some(next) = self.to_load.pop() {
            self.loaded.push(next);
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
pub struct Client {
    server_state: Arc<Mutex<ServerState>>,
    connection: Connection,
    name: String,
    uuid: UUID,
    keepalive_id: Option<i64>,
    keepalive_time: Instant,
    teleportation_ids: Vec<i32>,
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    pitch: f32,
    on_ground: bool,
    chunk_loader: ChunkLoader,
}

impl Client {
    pub fn initialize(
        server_state: Arc<Mutex<ServerState>>,
        login_player: ClientLogin,
    ) -> Result<Self> {
        let Some((name, uuid)) = &login_player.player else {
            return Err(anyhow!("Login player incomplete"));
        };
        let name = name.clone();
        let uuid = *uuid;

        println!("{} connected", name);

        let mut client = Client {
            server_state,
            connection: login_player.into_connection(),
            name,
            uuid,
            keepalive_id: None,
            keepalive_time: Instant::now(),
            teleportation_ids: Vec::new(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            chunk_loader: ChunkLoader::new(16.0),
        };

        {
            let server_state = client.server_state.lock().unwrap();
            client.connection.send(packet::play::LoginPlay {
                entity_id: 0,
                is_hardcore: false,
                dimensions: vec![server_state.world_main_name.clone()],
                max_players: 20,
                view_distance: 16,
                simulation_distance: 16,
                reduced_debug_info: false,
                enable_respawn_screen: true,
                do_limited_crafting: false,
                dimension_type: 0,
                dimension_name: server_state.world_main_name.clone(),
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

        client.teleport(client.x, client.y, client.z, client.yaw, client.pitch)?;

        client
            .connection
            .send(packet::play::GameEvent::StartWaitingForLevelChunks)?;

        client.connection.send(packet::play::SetCenterChunk {
            chunk_x: 0,
            chunk_z: 0,
        })?;

        Ok(client)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> &UUID {
        &self.uuid
    }

    pub fn disconnected(&self) -> bool {
        self.connection.is_closed()
    }

    pub fn teleport(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) -> Result<()> {
        let mut teleport_id: i32 = 0;
        while self.teleportation_ids.contains(&teleport_id) {
            teleport_id += 1;
        }
        self.teleportation_ids.push(teleport_id);
        self.connection
            .send(packet::play::SynchronizePlayerPosition {
                relative: false,
                x: Some(x),
                y: Some(y),
                z: Some(z),
                yaw: Some(yaw),
                pitch: Some(pitch),
                teleport_id,
            })?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        if Instant::now().duration_since(self.keepalive_time) >= KEEPALIVE_PING_TIME {
            self.keepalive_time = Instant::now();
            if self.keepalive_id.is_some() {
                return Err(anyhow!("Client didn't respond to keepalive in time."));
            }
            let id: i64 = thread_rng().gen();
            self.keepalive_id = Some(id);
            self.connection.send(packet::play::KeepAlive { id })?;
        }

        if let Some(raw_packet) = self.connection.recieve()? {
            match ClientPlayPacket::try_from(raw_packet)? {
                ClientPlayPacket::KeepAlive(keep_alive) => {
                    if let Some(expected_keep_alive_id) = self.keepalive_id {
                        if keep_alive.id == expected_keep_alive_id {
                            self.keepalive_id = None;
                        } else {
                            return Err(anyhow!("Client keepalive responded with wrong id"));
                        }
                    } else {
                        return Err(anyhow!("Client keepalive responded to invalid keepalive"));
                    }
                }
                ClientPlayPacket::ConfirmTeleport(confirm_teleport) => {
                    self.teleportation_ids
                        .retain(|id| *id != confirm_teleport.teleport_id);
                }
                ClientPlayPacket::SetPlayerPositionAndRotation(
                    set_player_position_and_rotation,
                ) => {
                    if self.teleportation_ids.is_empty() {
                        self.x = set_player_position_and_rotation.x;
                        self.y = set_player_position_and_rotation.y;
                        self.z = set_player_position_and_rotation.z;
                        self.on_ground = set_player_position_and_rotation.on_ground;
                    }
                    self.yaw = set_player_position_and_rotation.yaw;
                    self.pitch = set_player_position_and_rotation.pitch;
                }
                ClientPlayPacket::SetPlayerPosition(set_player_position) => {
                    if self.teleportation_ids.is_empty() {
                        self.x = set_player_position.x;
                        self.y = set_player_position.y;
                        self.z = set_player_position.z;
                        self.on_ground = set_player_position.on_ground;
                    }
                }
                ClientPlayPacket::SetPlayerRotation(set_player_rotation) => {
                    if self.teleportation_ids.is_empty() {
                        self.on_ground = set_player_rotation.on_ground;
                    }
                    self.yaw = set_player_rotation.yaw;
                    self.pitch = set_player_rotation.pitch;
                }
            }
        }

        let chunk_position = ChunkPosition {
            chunk_x: (self.x as i32) / 16,
            chunk_z: (self.z as i32) / 16,
        };
        if self.chunk_loader.update_center(chunk_position) {
            self.connection.send(packet::play::SetCenterChunk {
                chunk_x: chunk_position.chunk_x,
                chunk_z: chunk_position.chunk_z,
            })?;

            while let Some(to_unload) = self.chunk_loader.next_to_unload() {
                println!("UNLOAD: {:?}", to_unload);
                self.connection.send(packet::play::UnloadChunk {
                    chunk_x: to_unload.chunk_x,
                    chunk_z: to_unload.chunk_z,
                })?;
            }

            let server_state = self.server_state.lock().unwrap();
            let world_height: usize =
                (server_state.world_max_y - server_state.world_min_y).try_into()?;
            if world_height % 16 != 0 {
                panic!("Invalid world height.");
            }
            let num_sections = world_height / 16;

            while let Some(to_load) = self.chunk_loader.next_to_load() {
                println!("LOAD: {:?}", to_load);
                self.connection
                    .send(packet::play::ChunkDataAndUpdateLight {
                        chunk_x: to_load.chunk_x,
                        chunk_z: to_load.chunk_z,
                        heightmaps: nbt_compound!(),
                        data: {
                            let mut writer = PacketWriter::new_empty();

                            #[derive(Eq, PartialEq, Hash, Clone, Copy)]
                            struct Air;
                            impl Paletteable for Air {
                                fn palette_value(&self) -> Result<i32> {
                                    Ok(0)
                                }
                            }

                            for _ in 0..num_sections {
                                writer.write_short(0)?;
                                writer.write_buf(&to_paletted_container(&[Air; 4096], 4, 8)?)?;
                                // Biome??
                                writer.write_buf(&to_paletted_container(&[Air; 64], 1, 3)?)?;
                            }

                            writer.into_inner().into_boxed_slice()
                        },
                        block_entities: Vec::new(),
                        // Empty lighting data for now.
                        sky_light_mask: BitSet::new(num_sections + 2),
                        block_light_mask: BitSet::new(num_sections + 2),
                        empty_sky_light_mask: BitSet::new(num_sections + 2),
                        empty_block_light_mask: BitSet::new(num_sections + 2),
                        sky_lights_arrays: Vec::new(),
                        block_lights_arrays: Vec::new(),
                    })?;
            }
        }

        Ok(())
    }
}
