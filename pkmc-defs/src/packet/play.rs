use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{Read, Write},
};

use pkmc_util::{
    nbt::NBT,
    packet::{
        to_paletted_data_singular, ClientboundPacket, ConnectionError, PacketDecoder as _,
        PacketEncoder as _, ServerboundPacket,
    },
    serverbound_packet_enum, BitSet, FixedBitSet, Position, ReadExt as _, Transmutable, Vec3, UUID,
};

use crate::{
    particle::{self, Particle},
    text_component::TextComponent,
};

#[derive(Debug)]
pub struct BundleDelimiter;

impl ClientboundPacket for BundleDelimiter {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_BUNDLE_DELIMITER;

    fn packet_write(&self, _writer: impl Write) -> Result<(), ConnectionError> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Login {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub dimensions: Vec<String>,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub enable_respawn_screen: bool,
    pub do_limited_crafting: bool,
    pub dimension_type: i32,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub game_mode: u8,
    pub previous_game_mode: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub death: Option<(String, Position)>,
    pub portal_cooldown: i32,
    pub sea_level: i32,
    pub enforces_secure_chat: bool,
}

impl ClientboundPacket for Login {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_LOGIN;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.entity_id.to_be_bytes())?;
        writer.encode(self.is_hardcore)?;
        writer.encode(self.dimensions.len() as i32)?;
        self.dimensions.iter().try_for_each(|s| writer.encode(s))?;
        writer.encode(self.max_players)?;
        writer.encode(self.view_distance)?;
        writer.encode(self.simulation_distance)?;
        writer.encode(self.reduced_debug_info)?;
        writer.encode(self.enable_respawn_screen)?;
        writer.encode(self.do_limited_crafting)?;
        writer.encode(self.dimension_type)?;
        writer.encode(&self.dimension_name)?;
        writer.write_all(&self.hashed_seed.to_be_bytes())?;
        writer.write_all(&self.game_mode.to_be_bytes())?;
        writer.write_all(&self.previous_game_mode.to_be_bytes())?;
        writer.encode(self.is_debug)?;
        writer.encode(self.is_flat)?;
        if let Some(death) = &self.death {
            writer.encode(true)?;
            writer.encode(&death.0)?;
            writer.encode(&death.1)?;
        } else {
            writer.encode(false)?;
        }
        writer.encode(self.portal_cooldown)?;
        writer.encode(self.sea_level)?;
        writer.encode(self.enforces_secure_chat)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Disconnect(pub TextComponent);

impl ClientboundPacket for Disconnect {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_DISCONNECT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.0.to_nbt())?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum GameEvent {
    ChangeGamemode(u8),
    StartWaitingForLevelChunks,
}

impl ClientboundPacket for GameEvent {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_GAME_EVENT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        match self {
            GameEvent::ChangeGamemode(gamemode) => {
                writer.write_all(&3u8.to_be_bytes())?;
                writer.write_all(&(*gamemode as f32).to_be_bytes())?;
            }
            GameEvent::StartWaitingForLevelChunks => {
                writer.write_all(&13u8.to_be_bytes())?;
                writer.write_all(&0.0f32.to_be_bytes())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct KeepAlive {
    pub id: i64,
}

impl ClientboundPacket for KeepAlive {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_KEEP_ALIVE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.id.to_be_bytes())?;
        Ok(())
    }
}

impl ServerboundPacket for KeepAlive {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_KEEP_ALIVE;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: i64::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct PlayerLoaded;

impl ServerboundPacket for PlayerLoaded {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_LOADED;

    fn packet_read(_reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Debug, Default)]
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pos_relative: bool,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
    pub delta_relative: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub angle_relative: bool,
    pub rotate_delta: bool,
    pub teleport_id: i32,
}

impl ClientboundPacket for PlayerPosition {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_POSITION;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.teleport_id)?;
        writer.write_all(&self.x.to_be_bytes())?;
        writer.write_all(&self.y.to_be_bytes())?;
        writer.write_all(&self.z.to_be_bytes())?;
        writer.write_all(&self.delta_x.to_be_bytes())?;
        writer.write_all(&self.delta_y.to_be_bytes())?;
        writer.write_all(&self.delta_z.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.write_all(
            &(if self.pos_relative { 0b111i32 } else { 0 }
                | if self.angle_relative { 0b11000 } else { 0 }
                | if self.delta_relative { 0b11100000 } else { 0 }
                | if self.rotate_delta { 0b100000000 } else { 0 })
            .to_be_bytes(),
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct AcceptTeleportation {
    pub teleport_id: i32,
}

impl ServerboundPacket for AcceptTeleportation {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_ACCEPT_TELEPORTATION;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            teleport_id: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerPosRot {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerPosRot {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_POS_ROT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            x: f64::from_be_bytes(reader.read_const()?),
            y: f64::from_be_bytes(reader.read_const()?),
            z: f64::from_be_bytes(reader.read_const()?),
            yaw: f32::from_be_bytes(reader.read_const()?),
            pitch: f32::from_be_bytes(reader.read_const()?),
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerPos {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_POS;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            x: f64::from_be_bytes(reader.read_const()?),
            y: f64::from_be_bytes(reader.read_const()?),
            z: f64::from_be_bytes(reader.read_const()?),
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerRot {
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerRot {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_ROT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            yaw: f32::from_be_bytes(reader.read_const()?),
            pitch: f32::from_be_bytes(reader.read_const()?),
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerStatusOnly {
    pub flags: u8,
}

impl ServerboundPacket for MovePlayerStatusOnly {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_MOVE_PLAYER_STATUS_ONLY;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct ClientTickEnd;

impl ServerboundPacket for ClientTickEnd {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_CLIENT_TICK_END;

    fn packet_read(_reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self)
    }
}

#[derive(Debug)]
pub struct PlayerInput {
    pub flags: u8,
}

impl ServerboundPacket for PlayerInput {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_INPUT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug)]
pub struct SetChunkCacheCenter {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for SetChunkCacheCenter {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.chunk_x)?;
        writer.encode(self.chunk_z)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BlockEntity {
    /// u4
    pub x: u8,
    /// u4
    pub z: u8,
    pub y: i16,
    pub r#type: i32,
    pub data: NBT,
}

#[derive(Debug)]
pub struct LevelChunkData {
    pub heightmaps: NBT,
    pub data: Box<[u8]>,
    pub block_entities: Vec<BlockEntity>,
}

impl LevelChunkData {
    fn write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.heightmaps)?;
        writer.encode(self.data.len() as i32)?;
        writer.write_all(&self.data)?;
        writer.encode(self.block_entities.len() as i32)?;
        for block_entity in self.block_entities.iter() {
            debug_assert!(block_entity.x <= 15);
            debug_assert!(block_entity.z <= 15);
            writer.write_all(&((block_entity.x << 4) | block_entity.z).to_be_bytes())?;
            writer.write_all(&block_entity.y.to_be_bytes())?;
            writer.encode(block_entity.r#type)?;
            writer.encode(&block_entity.data)?;
        }
        //println!("{:#?}", self.block_entities);
        Ok(())
    }
}

#[derive(Debug)]
pub struct LevelLightData {
    pub num_sections: usize,
    pub sky_lights_arrays: Box<[Option<[u8; 2048]>]>,
    pub block_lights_arrays: Box<[Option<[u8; 2048]>]>,
}

impl LevelLightData {
    pub fn full_dark(num_sections: usize) -> Self {
        Self {
            num_sections,
            sky_lights_arrays: vec![None; num_sections + 2].into_boxed_slice(),
            block_lights_arrays: vec![None; num_sections + 2].into_boxed_slice(),
        }
    }

    pub fn full_bright(num_sections: usize) -> Self {
        Self {
            num_sections,
            sky_lights_arrays: vec![Some([0xFF; 2048]); num_sections + 2].into_boxed_slice(),
            block_lights_arrays: vec![Some([0xFF; 2048]); num_sections + 2].into_boxed_slice(),
        }
    }

    fn write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        assert_eq!(self.sky_lights_arrays.len(), self.num_sections + 2);
        assert_eq!(self.block_lights_arrays.len(), self.num_sections + 2);

        let mut sky_light_bitset = BitSet::new(self.num_sections + 2);
        self.sky_lights_arrays
            .iter()
            .enumerate()
            .for_each(|(i, a)| sky_light_bitset.set(i, a.is_some()));
        writer.encode(&sky_light_bitset)?;

        let mut block_light_bitset = BitSet::new(self.num_sections + 2);
        self.block_lights_arrays
            .iter()
            .enumerate()
            .for_each(|(i, a)| block_light_bitset.set(i, a.is_some()));
        writer.encode(&block_light_bitset)?;

        // ?????
        writer.encode(0)?;
        writer.encode(0)?;

        writer.encode(self.sky_lights_arrays.iter().flatten().count() as i32)?;
        for sky_light_array in self.sky_lights_arrays.iter().flatten() {
            writer.encode(2048)?;
            writer.write_all(sky_light_array)?;
        }

        writer.encode(self.block_lights_arrays.iter().flatten().count() as i32)?;
        for block_light_array in self.block_lights_arrays.iter().flatten() {
            writer.encode(2048)?;
            writer.write_all(block_light_array)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct LevelChunkWithLight {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub chunk_data: LevelChunkData,
    pub light_data: LevelLightData,
}

impl LevelChunkWithLight {
    pub fn generate_test(chunk_x: i32, chunk_z: i32, num_sections: usize) -> std::io::Result<Self> {
        Ok(Self {
            chunk_x,
            chunk_z,
            chunk_data: LevelChunkData {
                heightmaps: NBT::Compound(HashMap::new()),
                data: {
                    let mut writer = Vec::new();

                    for i in 0..num_sections {
                        let block_id = if i == 0 { 1 } else { 0 };
                        // Num non-air blocks
                        writer.write_all(
                            &if pkmc_generated::block::is_air(block_id) {
                                0i16
                            } else {
                                4096i16
                            }
                            .to_be_bytes(),
                        )?;

                        // Blocks
                        writer.write_all(&to_paletted_data_singular(block_id)?)?;
                        // Biome
                        writer.write_all(&to_paletted_data_singular(0)?)?;
                    }

                    writer.into_boxed_slice()
                },
                block_entities: Vec::new(),
            },
            light_data: LevelLightData::full_dark(num_sections),
        })
    }
}

impl ClientboundPacket for LevelChunkWithLight {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.chunk_x.to_be_bytes())?;
        writer.write_all(&self.chunk_z.to_be_bytes())?;
        self.chunk_data.write(&mut writer)?;
        self.light_data.write(&mut writer)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ForgetLevelChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for ForgetLevelChunk {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_FORGET_LEVEL_CHUNK;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.chunk_z.to_be_bytes())?;
        writer.write_all(&self.chunk_x.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct PlayerAbilities_Clientbound {
    pub flags: u8,
    pub flying_speed: f32,
    pub field_of_view_modifier: f32,
}

impl ClientboundPacket for PlayerAbilities_Clientbound {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_ABILITIES;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.flags.to_be_bytes())?;
        writer.write_all(&self.flying_speed.to_be_bytes())?;
        writer.write_all(&self.field_of_view_modifier.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct PlayerAbilities_Serverbound {
    pub flags: u8,
}

impl ServerboundPacket for PlayerAbilities_Serverbound {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_ABILITIES;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            flags: u8::from_be_bytes(reader.read_const()?),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCommandAction {
    StartSneaking,
    StopSneaking,
    LeaveBed,
    StartSprinting,
    StopSprinting,
    StartJumpWithHorse,
    StopJumpWithHorse,
    OpenVehicleInventory,
    StartFlyingWithElytra,
}

impl TryFrom<i32> for PlayerCommandAction {
    type Error = ConnectionError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PlayerCommandAction::StartSneaking),
            1 => Ok(PlayerCommandAction::StopSneaking),
            2 => Ok(PlayerCommandAction::LeaveBed),
            3 => Ok(PlayerCommandAction::StartSprinting),
            4 => Ok(PlayerCommandAction::StopSprinting),
            5 => Ok(PlayerCommandAction::StartJumpWithHorse),
            6 => Ok(PlayerCommandAction::StopJumpWithHorse),
            7 => Ok(PlayerCommandAction::OpenVehicleInventory),
            8 => Ok(PlayerCommandAction::StartFlyingWithElytra),
            _ => Err(ConnectionError::Other(
                "packet::play::PlayerActionCommand invalid varint value".into(),
            )),
        }
    }
}

#[derive(Debug)]
pub struct PlayerCommand {
    pub entity_id: i32,
    pub action: PlayerCommandAction,
    pub jump_boost: i32,
}

impl ServerboundPacket for PlayerCommand {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_PLAYER_COMMAND;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            entity_id: reader.decode()?,
            action: PlayerCommandAction::try_from(reader.decode::<i32>()?)?,
            jump_boost: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct SystemChat {
    pub content: TextComponent,
    pub overlay: bool,
}

impl ClientboundPacket for SystemChat {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SYSTEM_CHAT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.content.to_nbt())?;
        writer.encode(self.overlay)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetActionBarText(pub TextComponent);

impl ClientboundPacket for SetActionBarText {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_ACTION_BAR_TEXT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.0.to_nbt())?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ServerLink {
    BugReport,
    CommunityGuidelines,
    Support,
    Status,
    Feedback,
    Community,
    Website,
    Forums,
    News,
    Announcements,
    Custom(TextComponent),
}

impl ServerLink {
    fn write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(!matches!(self, ServerLink::Custom(..)))?;
        match self {
            ServerLink::BugReport => writer.encode(0)?,
            ServerLink::CommunityGuidelines => writer.encode(1)?,
            ServerLink::Support => writer.encode(2)?,
            ServerLink::Status => writer.encode(3)?,
            ServerLink::Feedback => writer.encode(4)?,
            ServerLink::Community => writer.encode(5)?,
            ServerLink::Website => writer.encode(6)?,
            ServerLink::Forums => writer.encode(7)?,
            ServerLink::News => writer.encode(8)?,
            ServerLink::Announcements => writer.encode(9)?,
            ServerLink::Custom(text_component) => writer.encode(&text_component.to_nbt())?,
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ServerLinks {
    pub links: Vec<(ServerLink, String)>,
}

impl ServerLinks {
    pub fn new<S, I>(links: I) -> Self
    where
        S: Into<String>,
        I: IntoIterator<Item = (ServerLink, S)>,
    {
        Self {
            links: links
                .into_iter()
                .map(|(link, url)| (link, url.into()))
                .collect(),
        }
    }
}

impl ClientboundPacket for ServerLinks {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SERVER_LINKS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.links.len() as i32)?;
        for (link, url) in &self.links {
            link.write(&mut writer)?;
            writer.encode(url)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetCarriedItem(pub u16);

impl ServerboundPacket for SetCarriedItem {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_SET_CARRIED_ITEM;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self(u16::from_be_bytes(reader.read_const()?)))
    }
}

#[derive(Debug)]
pub struct SetChunkChacheRadius(pub i32);

impl ClientboundPacket for SetChunkChacheRadius {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.0)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SwingArm(pub bool);

impl ServerboundPacket for SwingArm {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_SWING;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self(match reader.decode::<i32>()? {
            0 => false,
            1 => true,
            _ => return Err(ConnectionError::Other("Invalid swing arm.".into())),
        }))
    }
}

#[derive(Debug)]
pub struct UpdateSectionBlocks {
    pub section: Position,
    pub blocks: Vec<(u8, u8, u8, i32)>,
}

impl ClientboundPacket for UpdateSectionBlocks {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        //writer.write_position(&self.section)?;
        let v: u64 = Transmutable::<u64>::transmute((self.section.x as i64) << 42)
            | (Transmutable::<u64>::transmute((self.section.y as i64) << 44) >> 44)
            | (Transmutable::<u64>::transmute((self.section.z as i64) << 42) >> 22);
        writer.write_all(&v.to_be_bytes())?;

        writer.encode(self.blocks.len() as i32)?;
        for (bx, by, bz, id) in self.blocks.iter() {
            debug_assert!(*bx <= 15);
            debug_assert!(*by <= 15);
            debug_assert!(*bz <= 15);
            let encoded_position: u64 = ((*bx as u64) << 8) | ((*bz as u64) << 4) | (*by as u64);
            writer
                .encode(((*id as i64) << 12) | Transmutable::<i64>::transmute(encoded_position))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct AddEntity {
    pub id: i32,
    pub uuid: UUID,
    pub r#type: i32,
    pub position: Vec3<f64>,
    pub pitch: u8,
    pub yaw: u8,
    pub head_yaw: u8,
    pub data: i32,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub velocity_z: i16,
}

impl ClientboundPacket for AddEntity {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ADD_ENTITY;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.id)?;
        writer.encode(&self.uuid)?;
        writer.encode(self.r#type)?;
        writer.write_all(&self.position.x.to_be_bytes())?;
        writer.write_all(&self.position.y.to_be_bytes())?;
        writer.write_all(&self.position.z.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.head_yaw.to_be_bytes())?;
        writer.encode(self.data)?;
        writer.write_all(&self.velocity_x.to_be_bytes())?;
        writer.write_all(&self.velocity_y.to_be_bytes())?;
        writer.write_all(&self.velocity_z.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityMetadata {
    Byte(u8),
    VarInt(i32),
    VarLong(i64),
    Float(f32),
    String(String),
    TextComponent(TextComponent),
    OptionalTextComponent(Option<TextComponent>),
    /// UNIMPLEMENTED
    Slot,
    Boolean(bool),
    Rotations(f32, f32, f32),
    Position(Position),
    OptionalPosition(Option<Position>),
    // TODO: Implement enum
    Direction(i32),
    OptionalUUID(Option<UUID>),
    BlockState(i32),
    OptionalBlockState(Option<i32>),
    NBT(NBT),
    Particle(i32),
    Particles(Vec<i32>),
    VillagerData(i32, i32, i32),
    OptionalVarInt(Option<i32>),
    // TODO: Implement enum
    Pose(i32),
    CatVariant(i32),
    /// UNIMPLEMENTED
    WolfVariant(i32),
    FrogVariant(i32),
    OptionalGlobalPosition(Option<(String, Position)>),
    /// UNIMPLEMENTED
    PaintingVariant,
    /// UNIMPLEMENTED
    SnifferState(i32),
    /// UNIMPLEMENTED
    ArmadilloState(i32),
    Vector3(Vec3<f32>),
    // TEMP: Implement quaternions
    Quaternion(f32, f32, f32, f32),
}

impl EntityMetadata {
    pub fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
        match self {
            EntityMetadata::Byte(byte) => {
                writer.encode(0)?;
                writer.write_all(&byte.to_be_bytes())?;
            }
            EntityMetadata::VarInt(varint) => {
                writer.encode(1)?;
                writer.encode(*varint)?;
            }
            EntityMetadata::VarLong(varlong) => {
                writer.encode(2)?;
                writer.encode(*varlong)?;
            }
            EntityMetadata::Float(float) => {
                writer.encode(3)?;
                writer.write_all(&float.to_be_bytes())?;
            }
            EntityMetadata::String(string) => {
                writer.encode(4)?;
                writer.encode(string)?;
            }
            EntityMetadata::TextComponent(text_component) => {
                writer.encode(5)?;
                writer.encode(&text_component.to_nbt())?;
            }
            EntityMetadata::OptionalTextComponent(text_component) => {
                writer.encode(6)?;
                writer.encode(text_component.as_ref().map(|v| v.to_nbt()).as_ref())?;
            }
            EntityMetadata::Slot => todo!(),
            EntityMetadata::Boolean(bool) => {
                writer.encode(8)?;
                writer.encode(*bool)?;
            }
            EntityMetadata::Rotations(rx, ry, rz) => {
                writer.encode(9)?;
                writer.write_all(&rx.to_be_bytes())?;
                writer.write_all(&ry.to_be_bytes())?;
                writer.write_all(&rz.to_be_bytes())?;
            }
            EntityMetadata::Position(position) => {
                writer.encode(10)?;
                writer.encode(position)?;
            }
            EntityMetadata::OptionalPosition(position) => {
                writer.encode(11)?;
                writer.encode(position.as_ref())?;
            }
            EntityMetadata::Direction(direction) => {
                writer.encode(12)?;
                writer.encode(*direction)?;
            }
            EntityMetadata::OptionalUUID(uuid) => {
                writer.encode(13)?;
                writer.encode(uuid.as_ref())?;
            }
            EntityMetadata::BlockState(block_state) => {
                writer.encode(14)?;
                writer.encode(*block_state)?;
            }
            EntityMetadata::OptionalBlockState(block_state) => {
                writer.encode(15)?;
                if let Some(block_state) = block_state {
                    assert_ne!(*block_state, 0);
                    writer.encode(*block_state)?;
                } else {
                    writer.encode(0)?;
                }
            }
            EntityMetadata::NBT(nbt) => {
                writer.encode(16)?;
                writer.encode(nbt)?;
            }
            EntityMetadata::Particle(particle) => {
                writer.encode(17)?;
                writer.encode(*particle)?;
            }
            EntityMetadata::Particles(particles) => {
                writer.encode(18)?;
                writer.encode(particles.len() as i32)?;
                for particle in particles {
                    writer.encode(*particle)?;
                }
            }
            EntityMetadata::VillagerData(r#type, profession, level) => {
                writer.encode(19)?;
                writer.encode(*r#type)?;
                writer.encode(*profession)?;
                writer.encode(*level)?;
            }
            EntityMetadata::OptionalVarInt(var_int) => {
                writer.encode(20)?;
                if let Some(var_int) = var_int {
                    writer.encode(var_int.checked_add(1).ok_or(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Optional var int size too large",
                    ))?)?;
                } else {
                    writer.encode(0)?;
                }
            }
            EntityMetadata::Pose(pose) => {
                writer.encode(21)?;
                writer.encode(*pose)?;
            }
            EntityMetadata::CatVariant(cat_variant) => {
                writer.encode(22)?;
                writer.encode(*cat_variant)?;
            }
            EntityMetadata::WolfVariant(_wolf_variant) => todo!(),
            EntityMetadata::FrogVariant(frog_variant) => {
                writer.encode(24)?;
                writer.encode(*frog_variant)?;
            }
            EntityMetadata::OptionalGlobalPosition(global_position) => {
                writer.encode(25)?;
                if let Some((dimension, position)) = global_position {
                    writer.encode(true)?;
                    writer.encode(dimension)?;
                    writer.encode(position)?;
                } else {
                    writer.encode(false)?;
                }
            }
            EntityMetadata::PaintingVariant => todo!(),
            EntityMetadata::SnifferState(sniffer_state) => {
                writer.encode(27)?;
                writer.encode(*sniffer_state)?;
            }
            EntityMetadata::ArmadilloState(armadillo_state) => {
                writer.encode(28)?;
                writer.encode(*armadillo_state)?;
            }
            EntityMetadata::Vector3(vec3) => {
                writer.encode(29)?;
                writer.write_all(&vec3.x.to_be_bytes())?;
                writer.write_all(&vec3.y.to_be_bytes())?;
                writer.write_all(&vec3.z.to_be_bytes())?;
            }
            EntityMetadata::Quaternion(x, y, z, w) => {
                writer.encode(30)?;
                writer.write_all(&x.to_be_bytes())?;
                writer.write_all(&y.to_be_bytes())?;
                writer.write_all(&z.to_be_bytes())?;
                writer.write_all(&w.to_be_bytes())?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityMetadataBundle(pub BTreeMap<u8, EntityMetadata>);

impl EntityMetadataBundle {
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }
}

impl EntityMetadataBundle {
    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_flags(&mut self, flags: u8) {
        self.0.insert(0, EntityMetadata::Byte(flags));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_air_ticks(&mut self, air_ticks: i32) {
        self.0.insert(1, EntityMetadata::VarInt(air_ticks));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_custom_name(&mut self, custom_name: Option<TextComponent>) {
        self.0
            .insert(2, EntityMetadata::OptionalTextComponent(custom_name));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_is_custom_name_visible(&mut self, is_custom_name_visible: bool) {
        self.0
            .insert(3, EntityMetadata::Boolean(is_custom_name_visible));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_is_silent(&mut self, is_silent: bool) {
        self.0.insert(4, EntityMetadata::Boolean(is_silent));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_has_no_gravity(&mut self, has_no_gravity: bool) {
        self.0.insert(5, EntityMetadata::Boolean(has_no_gravity));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_pose(&mut self, pose: i32) {
        self.0.insert(6, EntityMetadata::Pose(pose));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_ticks_frozen_in_powdered_snow(&mut self, ticks_frozen_in_powdered_snow: i32) {
        self.0
            .insert(7, EntityMetadata::VarInt(ticks_frozen_in_powdered_snow));
    }

    /// Entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Entity
    pub fn entity_default() -> Self {
        let mut bundle = Self::empty();
        bundle.entity_flags(0);
        bundle.entity_air_ticks(300);
        bundle.entity_custom_name(None);
        bundle.entity_is_custom_name_visible(false);
        bundle.entity_is_silent(false);
        bundle.entity_has_no_gravity(false);
        bundle.entity_pose(0);
        bundle.entity_ticks_frozen_in_powdered_snow(0);
        bundle
    }
}

impl EntityMetadataBundle {
    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_flags(&mut self, flags: u8) {
        self.0.insert(8, EntityMetadata::Byte(flags));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_health(&mut self, health: f32) {
        self.0.insert(9, EntityMetadata::Float(health));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_potion_effect_color(&mut self, potion_effect_color: Vec<i32>) {
        self.0
            .insert(10, EntityMetadata::Particles(potion_effect_color));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_potion_effect_is_ambient(&mut self, potion_effect_is_ambient: bool) {
        self.0
            .insert(11, EntityMetadata::Boolean(potion_effect_is_ambient));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_num_arrows_in_entity(&mut self, num_arrows_in_entity: i32) {
        self.0
            .insert(12, EntityMetadata::VarInt(num_arrows_in_entity));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_num_bee_stingers_in_entity(&mut self, num_bee_stingers_in_entity: i32) {
        self.0
            .insert(13, EntityMetadata::VarInt(num_bee_stingers_in_entity));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_sleeping_bed_position(&mut self, sleeping_bed_position: Option<Position>) {
        self.0
            .insert(14, EntityMetadata::OptionalPosition(sleeping_bed_position));
    }

    /// Living entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Living_Entity
    pub fn living_entity_default() -> Self {
        let mut bundle = Self::entity_default();
        bundle.living_entity_flags(0);
        bundle.living_entity_health(1.0);
        bundle.living_entity_potion_effect_color(Vec::new());
        bundle.living_entity_potion_effect_is_ambient(false);
        bundle.living_entity_num_arrows_in_entity(0);
        bundle.living_entity_num_bee_stingers_in_entity(0);
        bundle.living_entity_sleeping_bed_position(None);
        bundle
    }
}

impl EntityMetadataBundle {
    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_additional_hearts(&mut self, additional_hearts: f32) {
        self.0.insert(15, EntityMetadata::Float(additional_hearts));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_score(&mut self, score: i32) {
        self.0.insert(16, EntityMetadata::VarInt(score));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_skin_parts(&mut self, skin_parts: u8) {
        self.0.insert(17, EntityMetadata::Byte(skin_parts));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_main_hand(&mut self, main_hand: u8) {
        self.0.insert(18, EntityMetadata::Byte(main_hand));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_left_parrot(&mut self, left_parrot: NBT) {
        self.0.insert(19, EntityMetadata::NBT(left_parrot));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_right_parrot(&mut self, right_parrot: NBT) {
        self.0.insert(20, EntityMetadata::NBT(right_parrot));
    }

    /// Player entity metadata, SEE: https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Entity_metadata#Player
    pub fn player_default() -> Self {
        let mut bundle = Self::living_entity_default();
        bundle.player_additional_hearts(0.0);
        bundle.player_score(0);
        bundle.player_skin_parts(0);
        bundle.player_main_hand(0);
        bundle.player_left_parrot(NBT::Compound(HashMap::new()));
        bundle.player_right_parrot(NBT::Compound(HashMap::new()));
        bundle
    }
}

#[derive(Debug)]
pub struct SetEntityMetadata {
    pub entity_id: i32,
    pub metadata: EntityMetadataBundle,
}

impl ClientboundPacket for SetEntityMetadata {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_ENTITY_DATA;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        for (index, data) in &self.metadata.0 {
            writer.write_all(&index.to_be_bytes())?;
            data.write(&mut writer)?;
        }
        writer.write_all(&0xFFu8.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EntityPositionSync {
    pub entity_id: i32,
    pub position: Vec3<f64>,
    pub velocity: Vec3<f64>,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ClientboundPacket for EntityPositionSync {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ENTITY_POSITION_SYNC;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.position.x.to_be_bytes())?;
        writer.write_all(&self.position.y.to_be_bytes())?;
        writer.write_all(&self.position.z.to_be_bytes())?;
        writer.write_all(&self.velocity.x.to_be_bytes())?;
        writer.write_all(&self.velocity.y.to_be_bytes())?;
        writer.write_all(&self.velocity.z.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MoveEntityPos {
    pub entity_id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub on_ground: bool,
}

impl ClientboundPacket for MoveEntityPos {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_MOVE_ENTITY_POS;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.delta_x.to_be_bytes())?;
        writer.write_all(&self.delta_y.to_be_bytes())?;
        writer.write_all(&self.delta_z.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MoveEntityPosRot {
    pub entity_id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

impl ClientboundPacket for MoveEntityPosRot {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.delta_x.to_be_bytes())?;
        writer.write_all(&self.delta_y.to_be_bytes())?;
        writer.write_all(&self.delta_z.to_be_bytes())?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct MoveEntityRot {
    pub entity_id: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

impl ClientboundPacket for MoveEntityRot {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_MOVE_ENTITY_ROT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        writer.write_all(&self.pitch.to_be_bytes())?;
        writer.encode(self.on_ground)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetHeadRotation {
    pub entity_id: i32,
    pub yaw: u8,
}

impl ClientboundPacket for SetHeadRotation {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ROTATE_HEAD;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.yaw.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityAnimationType {
    SwingMainArm,
    LeaveBed,
    SwingOffhand,
    CriticalEffect,
    MagicCriticalEffect,
}

impl EntityAnimationType {
    fn value(&self) -> u8 {
        match self {
            EntityAnimationType::SwingMainArm => 0,
            EntityAnimationType::LeaveBed => 1,
            EntityAnimationType::SwingOffhand => 2,
            EntityAnimationType::CriticalEffect => 3,
            EntityAnimationType::MagicCriticalEffect => 4,
        }
    }

    pub fn can_stack(&self) -> bool {
        matches!(
            self,
            EntityAnimationType::CriticalEffect | EntityAnimationType::MagicCriticalEffect
        )
    }
}

pub struct EntityAnimation {
    pub entity_id: i32,
    pub r#type: EntityAnimationType,
}

impl ClientboundPacket for EntityAnimation {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_ANIMATE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.entity_id)?;
        writer.write_all(&self.r#type.value().to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct RemoveEntities(pub HashSet<i32>);

impl ClientboundPacket for RemoveEntities {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_REMOVE_ENTITIES;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.0.len() as i32)?;
        for id in &self.0 {
            writer.encode(*id)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum PlayerInfoUpdateAction {
    AddPlayer {
        name: String,
        properties: HashMap<String, (String, Option<String>)>,
    },
    // TODO:
    InitializeChat,
    UpdateGamemode(i32),
    UpdateListed(bool),
    UpdateLatency(i32),
    UpdateDisplayName(Option<TextComponent>),
    UpdateListPriority(i32),
    UpdateHat(bool),
}

impl PlayerInfoUpdateAction {
    fn flag(&self) -> u8 {
        match self {
            PlayerInfoUpdateAction::AddPlayer { .. } => 0x01,
            PlayerInfoUpdateAction::InitializeChat => 0x02,
            PlayerInfoUpdateAction::UpdateGamemode(_) => 0x04,
            PlayerInfoUpdateAction::UpdateListed(_) => 0x08,
            PlayerInfoUpdateAction::UpdateLatency(_) => 0x10,
            PlayerInfoUpdateAction::UpdateDisplayName(_) => 0x20,
            PlayerInfoUpdateAction::UpdateListPriority(_) => 0x40,
            PlayerInfoUpdateAction::UpdateHat(_) => 0x80,
        }
    }
}

#[derive(Debug)]
pub struct PlayerInfoUpdate(pub HashMap<UUID, Vec<PlayerInfoUpdateAction>>);

impl ClientboundPacket for PlayerInfoUpdate {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_INFO_UPDATE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        // https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Protocol#Player_Info_Update
        let Some(first) = self.0.values().next() else {
            //return Err(ConnectionError::Other(
            //    "PlayerInfoUpdate cannot be empty.".into(),
            //));
            writer.write_all(&0u8.to_be_bytes())?;
            writer.encode(0)?;
            return Ok(());
        };
        let actions_flags = first.iter().fold(0, |f, a| f | a.flag());
        writer.write_all(&actions_flags.to_be_bytes())?;

        writer.encode(self.0.len() as i32)?;

        for (uuid, actions) in &self.0 {
            writer.encode(uuid)?;

            // Validate flags
            if Some(actions_flags)
                != actions
                    .iter()
                    .map(|a| a.flag())
                    .try_fold(0, |f, a| (f & a == 0).then_some(f | a))
            {
                return Err(ConnectionError::Other(
                    "PlayerInfoUpdate all player action types do not match.".into(),
                ));
            }

            let mut sorted_actions = actions.iter().collect::<Vec<_>>();
            sorted_actions.sort_by_key(|a| a.flag());

            for action in sorted_actions {
                match action {
                    PlayerInfoUpdateAction::AddPlayer { name, properties } => {
                        writer.encode(name)?;
                        writer.encode(properties.len() as i32)?;
                        for (key, (value, signature)) in properties {
                            writer.encode(key)?;
                            writer.encode(value)?;
                            writer.encode(signature.as_ref())?;
                        }
                    }
                    PlayerInfoUpdateAction::InitializeChat => {
                        writer.encode(false)?;
                    }
                    PlayerInfoUpdateAction::UpdateGamemode(gamemode) => {
                        writer.encode(*gamemode)?;
                    }
                    PlayerInfoUpdateAction::UpdateListed(listed) => {
                        writer.encode(*listed)?;
                    }
                    PlayerInfoUpdateAction::UpdateLatency(latency) => {
                        writer.encode(*latency)?;
                    }
                    PlayerInfoUpdateAction::UpdateDisplayName(display_name) => {
                        writer.encode(display_name.as_ref().map(|v| v.to_nbt()).as_ref())?;
                    }
                    PlayerInfoUpdateAction::UpdateListPriority(list_priority) => {
                        writer.encode(*list_priority)?;
                    }
                    PlayerInfoUpdateAction::UpdateHat(hat) => {
                        writer.encode(*hat)?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct PlayerInfoRemove(pub HashSet<UUID>);

impl ClientboundPacket for PlayerInfoRemove {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_PLAYER_INFO_REMOVE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.0.len() as i32)?;
        for uuid in &self.0 {
            writer.encode(uuid)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LevelParticles {
    pub long_distance: bool,
    pub always_visible: bool,
    pub position: Vec3<f64>,
    pub offset: Vec3<f32>,
    pub max_speed: f32,
    pub particle_count: i32,
    pub particle: Particle,
}

impl ClientboundPacket for LevelParticles {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_LEVEL_PARTICLES;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.long_distance)?;
        writer.encode(self.always_visible)?;
        writer.write_all(&self.position.x.to_be_bytes())?;
        writer.write_all(&self.position.y.to_be_bytes())?;
        writer.write_all(&self.position.z.to_be_bytes())?;
        writer.write_all(&self.offset.x.to_be_bytes())?;
        writer.write_all(&self.offset.y.to_be_bytes())?;
        writer.write_all(&self.offset.z.to_be_bytes())?;
        writer.write_all(&self.max_speed.to_be_bytes())?;
        writer.write_all(&self.particle_count.to_be_bytes())?;
        writer.encode(self.particle.r#type().to_id())?;
        match &self.particle {
            Particle::Block(block) => {
                writer.encode(block.id_with_default_fallback().unwrap())?;
            }
            Particle::BlockMarker(block) => {
                writer.encode(block.id_with_default_fallback().unwrap())?;
            }
            Particle::Dust { color, scale } => {
                writer.write_all(&color.to_argb8888(0).to_be_bytes())?;
                writer.write_all(&scale.to_be_bytes())?;
            }
            Particle::DustColorTransition { from, to, scale } => {
                writer.write_all(&from.to_argb8888(0).to_be_bytes())?;
                writer.write_all(&to.to_argb8888(0).to_be_bytes())?;
                writer.write_all(&scale.to_be_bytes())?;
            }
            Particle::EntityEffect { color, alpha } => {
                writer.write_all(&color.to_argb8888(*alpha).to_be_bytes())?;
            }
            Particle::FallingDust(block) => {
                writer.encode(block.id_with_default_fallback().unwrap())?;
            }
            Particle::SculkCharge { roll } => {
                writer.write_all(&roll.to_be_bytes())?;
            }
            Particle::Item => unimplemented!(),
            Particle::Vibration { source, ticks } => {
                match source {
                    particle::VibrationSource::Block(position) => {
                        writer.encode(0)?;
                        writer.encode(position)?;
                    }
                    particle::VibrationSource::Entity { id, eye_height } => {
                        writer.encode(1)?;
                        writer.encode(*id)?;
                        writer.write_all(&eye_height.to_be_bytes())?;
                    }
                }
                writer.encode(*ticks)?;
            }
            Particle::Trail {
                position,
                color,
                duration,
            } => {
                writer.write_all(&position.x.to_be_bytes())?;
                writer.write_all(&position.y.to_be_bytes())?;
                writer.write_all(&position.z.to_be_bytes())?;
                writer.write_all(&color.to_argb8888(0).to_be_bytes())?;
                writer.encode(*duration)?;
            }
            Particle::Shriek { delay } => {
                writer.encode(*delay)?;
            }
            Particle::DustPillar(block) => {
                writer.encode(block.id_with_default_fallback().unwrap())?;
            }
            Particle::BlockCrumble(block) => {
                writer.encode(block.id_with_default_fallback().unwrap())?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub message: String,
    pub timestamp: i64,
    pub salt: i64,
    pub signature: Option<[u8; 256]>,
    pub message_count: i32,
    pub acknowledged: FixedBitSet<20>,
}

impl ServerboundPacket for ChatMessage {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_CHAT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            message: reader.decode()?,
            timestamp: i64::from_be_bytes(reader.read_const()?),
            salt: i64::from_be_bytes(reader.read_const()?),
            signature: reader
                .decode::<bool>()?
                .then(|| reader.read_const())
                .transpose()?,
            message_count: reader.decode()?,
            acknowledged: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct DisguisedChatMessage {
    pub message: TextComponent,
    // TODO: minecraft:chat_type registry generated code enum
    pub chat_type: i32,
    pub sender_name: TextComponent,
    pub target_name: Option<TextComponent>,
}

impl ClientboundPacket for DisguisedChatMessage {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_DISGUISED_CHAT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        // FIXME: Minecraft doesn't like decoding this, and I have no idea why.
        writer.encode(&self.message.to_nbt())?;
        writer.encode(self.chat_type)?;
        writer.encode(&self.sender_name.to_nbt())?;
        if let Some(target_name) = &self.target_name {
            writer.encode(true)?;
            writer.encode(&target_name.to_nbt())?;
        } else {
            writer.encode(false)?;
        }
        Ok(())
    }
}

serverbound_packet_enum!(pub PlayPacket;
    KeepAlive, KeepAlive;
    PlayerLoaded, PlayerLoaded;
    AcceptTeleportation, AcceptTeleportation;
    MovePlayerPosRot, MovePlayerPosRot;
    MovePlayerPos, MovePlayerPos;
    MovePlayerRot, MovePlayerRot;
    MovePlayerStatusOnly, MovePlayerStatusOnly;
    ClientTickEnd, ClientTickEnd;
    PlayerInput, PlayerInput;
    PlayerAbilities_Serverbound, PlayerAbilities;
    PlayerCommand, PlayerCommand;
    SetCarriedItem, SetHeldItem;
    SwingArm, SwingArm;
    ChatMessage, ChatMessage;
);
