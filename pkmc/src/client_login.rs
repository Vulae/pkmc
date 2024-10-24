use anyhow::Result;
use pkmc_defs::packet;
use pkmc_nbt::{nbt_compound, NBT};
use pkmc_packet::{create_packet_enum, Connection};
use pkmc_util::UUID;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::server_state::ServerState;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ClientLoginState {
    Login,
    Configuration,
    Play,
}

create_packet_enum!(ClientLoginLoginPacket;
    packet::login::LoginStart, Start;
    packet::login::LoginAcknowledged, Acknowledged;
);

create_packet_enum!(ClientLoginConfigurationPacket;
    packet::login::LoginConfigurationClientInformation, ClientInformation;
    packet::login::LoginConfigurationPluginMessage, PluginMessage;
    packet::login::LoginConfigurationKnownPacks, KnownPacks;
    packet::login::LoginConfigurationFinish, Finish;
);

/// A player that is in the process of logging in.
#[derive(Debug)]
pub struct ClientLogin {
    server_state: Arc<Mutex<ServerState>>,
    connection: Connection,
    last_recv_configuration_time: Instant,
    send_final_configuration_packet: bool,
    pub state: ClientLoginState,
    pub player: Option<(String, UUID)>,
}

impl ClientLogin {
    pub fn new(server_state: Arc<Mutex<ServerState>>, connection: Connection) -> Self {
        Self {
            server_state,
            connection,
            last_recv_configuration_time: Instant::now(),
            send_final_configuration_packet: false,
            state: ClientLoginState::Login,
            player: None,
        }
    }

    pub fn into_connection(self) -> Connection {
        self.connection
    }

    pub fn state(&self) -> ClientLoginState {
        self.state
    }

    pub fn update(&mut self) -> Result<()> {
        if let Some(raw_packet) = self.connection.recieve()? {
            match self.state {
                ClientLoginState::Login => match ClientLoginLoginPacket::try_from(raw_packet)? {
                    ClientLoginLoginPacket::Start(login_start) => {
                        self.player = Some((login_start.name.clone(), login_start.uuid));
                        self.connection.send(packet::login::LoginSuccess {
                            uuid: login_start.uuid,
                            name: login_start.name,
                            properties: Vec::new(),
                            strict_error_handling: false,
                        })?;
                    }
                    ClientLoginLoginPacket::Acknowledged(_) => {
                        self.last_recv_configuration_time = Instant::now();
                        self.state = ClientLoginState::Configuration;
                        self.connection
                            .send(packet::login::LoginConfigurationKnownPacks {
                                packs: vec![packet::login::LoginConfigurationKnownPack {
                                    namespace: "minecraft:core".to_string(),
                                    id: "".to_string(),
                                    version: "1.21".to_string(),
                                }],
                            })?;
                        let server_state = self.server_state.lock().unwrap();
                        self.connection.send(
                            packet::login::LoginConfigurationPluginMessage::Brand(
                                server_state.server_brand.clone(),
                            ),
                        )?;
                    }
                },
                ClientLoginState::Configuration => {
                    self.last_recv_configuration_time = Instant::now();
                    match ClientLoginConfigurationPacket::try_from(raw_packet)? {
                        ClientLoginConfigurationPacket::ClientInformation(_) => {}
                        ClientLoginConfigurationPacket::PluginMessage(_) => {}
                        ClientLoginConfigurationPacket::KnownPacks(_) => {
                            let server_state = self.server_state.lock().unwrap();
                            self.connection.send(
                                packet::login::LoginConfigurationRegistryData {
                                    registry_id: "minecraft:dimension_type".to_string(),
                                    entries: vec![
                                        packet::login::LoginConfigurationRegistryDataEntry {
                                            entry_id: server_state.world_main_name.clone(),
                                            // TODO: Use nbt_compound![]
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
                                                    ("min_y", NBT::Int(server_state.world_min_y)),
                                                    ("height", NBT::Int(server_state.world_max_y)),
                                                    ("logical_height", NBT::Int(0)),
                                                    (
                                                        "infiniburn",
                                                        NBT::String(
                                                            "#minecraft:infiniburn_overworld"
                                                                .to_string(),
                                                        ),
                                                    ),
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
                                },
                            )?;
                            self.connection.send(
                                packet::login::LoginConfigurationRegistryData {
                                    registry_id: "minecraft:painting_variant".to_string(),
                                    entries: vec![
                                        packet::login::LoginConfigurationRegistryDataEntry {
                                            entry_id: "minecraft:earth".to_string(),
                                            data: Some(nbt_compound![
                                                "asset_id" => NBT::String("minecraft:earth".to_string()),
                                                "height" => NBT::Int(2),
                                                "width" => NBT::Int(2),
                                            ]),
                                        },
                                    ],
                                },
                            )?;
                            self.connection.send(
                                packet::login::LoginConfigurationRegistryData {
                                    registry_id: "minecraft:wolf_variant".to_string(),
                                    entries: vec![
                                        packet::login::LoginConfigurationRegistryDataEntry {
                                            entry_id: "minecraft:woods".to_string(),
                                            data: Some(nbt_compound![
                                                "angry_texture" => NBT::String("minecraft:entity/wolf/wolf_woods_angry".to_string()),
                                                "biomes" => NBT::String("minecraft:forest".to_string()),
                                                "tame_texture" => NBT::String("minecraft:entity/wolf/wolf_woods_tame".to_string()),
                                                "wild_texture" => NBT::String("minecraft:entity/wolf/wolf_woods".to_string()),
                                            ]),
                                        },
                                    ],
                                },
                            )?;
                            self.connection.send(
                                packet::login::LoginConfigurationRegistryData {
                                    registry_id: "minecraft:worldgen/biome".to_string(),
                                    entries: [
                                        "pkmc:test",
                                        "minecraft:badlands",
                                        "minecraft:bamboo_jungle",
                                        "minecraft:basalt_deltas",
                                        "minecraft:beach",
                                        "minecraft:birch_forest",
                                        "minecraft:cherry_grove",
                                        "minecraft:cold_ocean",
                                        "minecraft:crimson_forest",
                                        "minecraft:dark_forest",
                                        "minecraft:deep_cold_ocean",
                                        "minecraft:deep_dark",
                                        "minecraft:deep_frozen_ocean",
                                        "minecraft:deep_lukewarm_ocean",
                                        "minecraft:deep_ocean",
                                        "minecraft:desert",
                                        "minecraft:dripstone_caves",
                                        "minecraft:end_barrens",
                                        "minecraft:end_highlands",
                                        "minecraft:end_midlands",
                                        "minecraft:eroded_badlands",
                                        "minecraft:flower_forest",
                                        "minecraft:forest",
                                        "minecraft:frozen_ocean",
                                        "minecraft:frozen_peaks",
                                        "minecraft:frozen_river",
                                        "minecraft:grove",
                                        "minecraft:ice_spikes",
                                        "minecraft:jagged_peaks",
                                        "minecraft:jungle",
                                        "minecraft:lukewarm_ocean",
                                        "minecraft:lush_caves",
                                        "minecraft:mangrove_swamp",
                                        "minecraft:meadow",
                                        "minecraft:mushroom_fields",
                                        "minecraft:nether_wastes",
                                        "minecraft:ocean",
                                        "minecraft:old_growth_birch_forest",
                                        "minecraft:old_growth_pine_taiga",
                                        "minecraft:old_growth_spruce_taiga",
                                        "minecraft:plains",
                                        "minecraft:river",
                                        "minecraft:savanna",
                                        "minecraft:savanna_plateau",
                                        "minecraft:small_end_islands",
                                        "minecraft:snowy_beach",
                                        "minecraft:snowy_plains",
                                        "minecraft:snowy_slopes",
                                        "minecraft:snowy_taiga",
                                        "minecraft:soul_sand_valley",
                                        "minecraft:sparse_jungle",
                                        "minecraft:stony_peaks",
                                        "minecraft:stony_shore",
                                        "minecraft:sunflower_plains",
                                        "minecraft:swamp",
                                        "minecraft:taiga",
                                        "minecraft:the_end",
                                        "minecraft:the_void",
                                        "minecraft:warm_ocean",
                                        "minecraft:warped_forest",
                                        "minecraft:windswept_forest",
                                        "minecraft:windswept_gravelly_hills",
                                        "minecraft:windswept_hills",
                                        "minecraft:windswept_savanna",
                                        "minecraft:wooded_badlands",
                                    ]
                                    .into_iter()
                                    .map(|biome_name| {
                                        packet::login::LoginConfigurationRegistryDataEntry {
                                            entry_id: biome_name.to_string(),
                                            data: Some(nbt_compound![
                                                "has_precipitation" => NBT::Byte(0),
                                                "temperature" => NBT::Float(0.5),
                                                "downfall" => NBT::Float(0.5),
                                                "effects" => nbt_compound![
                                                    "fog_color" => NBT::Int(12638463),
                                                    "sky_color" => NBT::Int(8103167),
                                                    "water_color" => NBT::Int(4159204),
                                                    "water_fog_color" => NBT::Int(329011),
                                                ],
                                            ]),
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                                },
                            )?;
                            self.connection.send(
                                packet::login::LoginConfigurationRegistryData {
                                    registry_id: "minecraft:damage_type".to_string(),
                                    entries: [
                                        "minecraft:arrow",
                                        "minecraft:bad_respawn_point",
                                        "minecraft:cactus",
                                        "minecraft:campfire",
                                        "minecraft:cramming",
                                        "minecraft:dragon_breath",
                                        "minecraft:drown",
                                        "minecraft:dry_out",
                                        "minecraft:explosion",
                                        "minecraft:fall",
                                        "minecraft:falling_anvil",
                                        "minecraft:falling_block",
                                        "minecraft:falling_stalactite",
                                        "minecraft:fireball",
                                        "minecraft:fireworks",
                                        "minecraft:fly_into_wall",
                                        "minecraft:freeze",
                                        "minecraft:generic",
                                        "minecraft:generic_kill",
                                        "minecraft:hot_floor",
                                        "minecraft:in_fire",
                                        "minecraft:in_wall",
                                        "minecraft:indirect_magic",
                                        "minecraft:lava",
                                        "minecraft:lightning_bolt",
                                        "minecraft:magic",
                                        "minecraft:mob_attack",
                                        "minecraft:mob_attack_no_aggro",
                                        "minecraft:mob_projectile",
                                        "minecraft:on_fire",
                                        "minecraft:out_of_world",
                                        "minecraft:outside_border",
                                        "minecraft:player_attack",
                                        "minecraft:player_explosion",
                                        "minecraft:sonic_boom",
                                        "minecraft:spit",
                                        "minecraft:stalagmite",
                                        "minecraft:starve",
                                        "minecraft:sting",
                                        "minecraft:sweet_berry_bush",
                                        "minecraft:thorns",
                                        "minecraft:thrown",
                                        "minecraft:trident",
                                        "minecraft:unattributed_fireball",
                                        "minecraft:wind_charge",
                                        "minecraft:wither",
                                        "minecraft:wither_skull",
                                    ]
                                    .into_iter()
                                    .map(|damage_type| {
                                        packet::login::LoginConfigurationRegistryDataEntry {
                                            entry_id: damage_type.to_string(),
                                            data: Some(nbt_compound![
                                                "message_id" => NBT::String("onFire".to_string()),
                                                "scaling" => NBT::String("never".to_string()),
                                                "exhaustion" => NBT::Float(0.0),
                                            ]),
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                                },
                            )?;
                        }
                        ClientLoginConfigurationPacket::Finish(_) => {
                            self.state = ClientLoginState::Play;
                        }
                    }
                }
                ClientLoginState::Play => unreachable!(),
            }
        }

        if self.state == ClientLoginState::Configuration
            && !self.send_final_configuration_packet
            && Instant::now().duration_since(self.last_recv_configuration_time)
                > Duration::from_millis(100)
        {
            self.send_final_configuration_packet = true;
            self.connection
                .send(packet::login::LoginConfigurationFinish)?;
        }

        Ok(())
    }
}
