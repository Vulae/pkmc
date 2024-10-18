use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{
    client_login::ClientLogin, connection::Connection, create_packet_enum, packet,
    server_state::ServerState, uuid::UUID,
};
use anyhow::{anyhow, Result};
use rand::{thread_rng, Rng};

create_packet_enum!(ClientPlayPacket;
    packet::play::KeepAlive, KeepAlive;
    packet::play::ConfirmTeleport, ConfirmTeleport;
    packet::play::SetPlayerPositionAndRotation, SetPlayerPositionAndRotation;
    packet::play::SetPlayerPosition, SetPlayerPosition;
    packet::play::SetPlayerRotation, SetPlayerRotation;
);

const KEEPALIVE_PING_TIME: Duration = Duration::from_millis(10000);

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
        Ok(())
    }
}
