use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use pkmc_defs::packet;
use pkmc_packet::{clientbound_packet_enum, Connection};
use pkmc_util::{IterRetain as _, UUID};
use rand::{thread_rng, Rng};

use crate::{client_login::ClientLogin, server_state::ServerState};

clientbound_packet_enum!(ClientPlayPacket;
    packet::play::KeepAlive, KeepAlive;
    packet::play::AcceptTeleportation, ConfirmTeleport;
    packet::play::MovePlayerPosRot, SetPlayerPositionAndRotation;
    packet::play::MovePlayerPos, SetPlayerPosition;
    packet::play::MovePlayerRot, SetPlayerRotation;
);

const KEEPALIVE_PING_TIME: Duration = Duration::from_millis(10000);

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

#[allow(unused)]
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

    fn visualize(&self) {
        //let min = self
        //    .loaded
        //    .iter()
        //    .fold(ChunkPosition::new(i32::MAX, i32::MAX), |min, chunk| {
        //        ChunkPosition::new(
        //            i32::min(min.chunk_x, chunk.chunk_x),
        //            i32::min(min.chunk_z, chunk.chunk_z),
        //        )
        //    });
        //let max = self
        //    .loaded
        //    .iter()
        //    .fold(ChunkPosition::new(i32::MIN, i32::MIN), |max, chunk| {
        //        ChunkPosition::new(
        //            i32::max(max.chunk_x, chunk.chunk_x),
        //            i32::max(max.chunk_z, chunk.chunk_z),
        //        )
        //    });
        let min = ChunkPosition::new(-10, -10);
        let max = ChunkPosition::new(10, 10);
        let mut grid: Vec<Vec<bool>> = (min.chunk_z..=max.chunk_z)
            .map(|_| (min.chunk_x..=max.chunk_x).map(|_| false).collect())
            .collect();
        self.loaded.iter().for_each(|chunk| {
            let grid_x = (chunk.chunk_x - min.chunk_x) as usize;
            let grid_z = (chunk.chunk_z - min.chunk_z) as usize;
            if let Some(row) = grid.get_mut(grid_z) {
                if let Some(cell) = row.get_mut(grid_x) {
                    *cell = true;
                }
            }
        });
        grid.into_iter().for_each(|row| {
            row.into_iter().for_each(|cell| {
                if cell {
                    print!("#");
                } else {
                    print!(" ");
                }
            });
            println!();
        });
    }
}

#[derive(Debug)]
pub struct Player {
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

impl Player {
    pub fn initialize(
        server_state: Arc<Mutex<ServerState>>,
        login_player: ClientLogin,
    ) -> Result<Self> {
        let Some((name, uuid)) = login_player.player.clone() else {
            return Err(anyhow!("Login player incomplete"));
        };

        let mut player = Player {
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
            chunk_loader: ChunkLoader::new(32),
        };

        //{
        //    let server_state = client.server_state.lock().unwrap();
        //    client.connection.send(packet::play::LoginPlay {
        //        entity_id: 0,
        //        is_hardcore: false,
        //        dimensions: vec![server_state.world_main_name.clone()],
        //        max_players: 20,
        //        view_distance: client.chunk_loader.radius,
        //        simulation_distance: 6,
        //        reduced_debug_info: false,
        //        enable_respawn_screen: true,
        //        do_limited_crafting: false,
        //        dimension_type: 0,
        //        dimension_name: server_state.world_main_name.clone(),
        //        hashed_seed: 0,
        //        game_mode: 1,
        //        previous_game_mode: -1,
        //        is_debug: false,
        //        is_flat: false,
        //        death: None,
        //        portal_cooldown: 0,
        //        sea_level: 0,
        //        enforces_secure_chat: false,
        //    })?;
        //}
        //
        //client.teleport(client.x, client.y, client.z, client.yaw, client.pitch)?;
        //
        //client
        //    .connection
        //    .send(packet::play::GameEvent::StartWaitingForLevelChunks)?;
        //
        //client.connection.send(packet::play::SetCenterChunk {
        //    chunk_x: 0,
        //    chunk_z: 0,
        //})?;
        //
        //client.connection.send(packet::play::PlayerAbilities {
        //    flags: 0x01 | 0x02 | 0x04 | 0x08,
        //    flying_speed: 0.5,
        //    field_of_view_modifier: 0.1,
        //})?;

        Ok(player)
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
        self.connection.send(packet::play::PlayerPosition {
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
        //if Instant::now().duration_since(self.keepalive_time) >= KEEPALIVE_PING_TIME {
        //    self.keepalive_time = Instant::now();
        //    if self.keepalive_id.is_some() {
        //        return Err(anyhow!("Client didn't respond to keepalive in time."));
        //    }
        //    let id: i64 = thread_rng().gen();
        //    self.keepalive_id = Some(id);
        //    self.connection.send(packet::play::KeepAlive { id })?;
        //}
        //
        //if let Some(raw_packet) = self.connection.recieve()? {
        //    match ClientPlayPacket::try_from(raw_packet)? {
        //        ClientPlayPacket::KeepAlive(keep_alive) => {
        //            if let Some(expected_keep_alive_id) = self.keepalive_id {
        //                if keep_alive.id == expected_keep_alive_id {
        //                    self.keepalive_id = None;
        //                } else {
        //                    return Err(anyhow!("Client keepalive responded with wrong id"));
        //                }
        //            } else {
        //                return Err(anyhow!("Client keepalive responded to invalid keepalive"));
        //            }
        //        }
        //        ClientPlayPacket::ConfirmTeleport(confirm_teleport) => {
        //            self.teleportation_ids
        //                .retain(|id| *id != confirm_teleport.teleport_id);
        //        }
        //        ClientPlayPacket::SetPlayerPositionAndRotation(
        //            set_player_position_and_rotation,
        //        ) => {
        //            if self.teleportation_ids.is_empty() {
        //                self.x = set_player_position_and_rotation.x;
        //                self.y = set_player_position_and_rotation.y;
        //                self.z = set_player_position_and_rotation.z;
        //                self.on_ground = set_player_position_and_rotation.on_ground;
        //            }
        //            self.yaw = set_player_position_and_rotation.yaw;
        //            self.pitch = set_player_position_and_rotation.pitch;
        //        }
        //        ClientPlayPacket::SetPlayerPosition(set_player_position) => {
        //            if self.teleportation_ids.is_empty() {
        //                self.x = set_player_position.x;
        //                self.y = set_player_position.y;
        //                self.z = set_player_position.z;
        //                self.on_ground = set_player_position.on_ground;
        //            }
        //        }
        //        ClientPlayPacket::SetPlayerRotation(set_player_rotation) => {
        //            if self.teleportation_ids.is_empty() {
        //                self.on_ground = set_player_rotation.on_ground;
        //            }
        //            self.yaw = set_player_rotation.yaw;
        //            self.pitch = set_player_rotation.pitch;
        //        }
        //    }
        //}
        //
        //let chunk_position = ChunkPosition {
        //    chunk_x: (self.x as i32) / 16,
        //    chunk_z: (self.z as i32) / 16,
        //};
        //if self.chunk_loader.update_center(Some(chunk_position)) {
        //    self.connection.send(packet::play::SetCenterChunk {
        //        chunk_x: chunk_position.chunk_x,
        //        chunk_z: chunk_position.chunk_z,
        //    })?;
        //}
        //while let Some(to_unload) = self.chunk_loader.next_to_unload() {
        //    //println!("UNLOAD: {:?}", to_unload);
        //    self.connection.send(packet::play::UnloadChunk {
        //        chunk_x: to_unload.chunk_x,
        //        chunk_z: to_unload.chunk_z,
        //    })?;
        //}
        //
        //let server_state = self.server_state.lock().unwrap();
        //let world_height: usize =
        //    (server_state.world_max_y - server_state.world_min_y).try_into()?;
        //if world_height % 16 != 0 {
        //    panic!("Invalid world height.");
        //}
        //let num_sections = world_height / 16;
        //
        //while let Some(to_load) = self.chunk_loader.next_to_load() {
        //    //println!("LOAD: {:?}", to_load);
        //    self.connection
        //        .send(packet::play::ChunkDataAndUpdateLight::generate_test(
        //            to_load.chunk_x,
        //            to_load.chunk_z,
        //            num_sections,
        //        )?)?;
        //}

        Ok(())
    }
}
