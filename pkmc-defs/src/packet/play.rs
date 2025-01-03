use std::io::{Read, Write};

use crate::{generated, text_component::TextComponent};
use pkmc_nbt::{nbt_compound, NBT};
use pkmc_packet::{
    connection::ConnectionError, serverbound_packet_enum, to_paletted_data, BitSet,
    ClientboundPacket, Position, ReadExtPacket, ServerboundPacket, WriteExtPacket,
};
use pkmc_util::read_ext::ReadExt;

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
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_LOGIN;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.entity_id.to_be_bytes())?;
        writer.write_bool(self.is_hardcore)?;
        writer.write_varint(self.dimensions.len() as i32)?;
        for dimension in self.dimensions.iter() {
            writer.write_string(dimension)?;
        }
        writer.write_varint(self.max_players)?;
        writer.write_varint(self.view_distance)?;
        writer.write_varint(self.simulation_distance)?;
        writer.write_bool(self.reduced_debug_info)?;
        writer.write_bool(self.enable_respawn_screen)?;
        writer.write_bool(self.do_limited_crafting)?;
        writer.write_varint(self.dimension_type)?;
        writer.write_string(&self.dimension_name)?;
        writer.write_all(&self.hashed_seed.to_be_bytes())?;
        writer.write_all(&self.game_mode.to_be_bytes())?;
        writer.write_all(&self.previous_game_mode.to_be_bytes())?;
        writer.write_bool(self.is_debug)?;
        writer.write_bool(self.is_flat)?;
        if let Some(death) = &self.death {
            writer.write_bool(true)?;
            writer.write_string(&death.0)?;
            writer.write_position(&death.1)?;
        } else {
            writer.write_bool(false)?;
        }
        writer.write_varint(self.portal_cooldown)?;
        writer.write_varint(self.sea_level)?;
        writer.write_bool(self.enforces_secure_chat)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Disconnect(pub TextComponent);

impl ClientboundPacket for Disconnect {
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_DISCONNECT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_nbt(&self.0.to_nbt())?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum GameEvent {
    StartWaitingForLevelChunks,
}

impl ClientboundPacket for GameEvent {
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_GAME_EVENT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        match self {
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
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_KEEP_ALIVE;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.id.to_be_bytes())?;
        Ok(())
    }
}

impl ServerboundPacket for KeepAlive {
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_KEEP_ALIVE;

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
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_PLAYER_LOADED;

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
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_PLAYER_POSITION;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_varint(self.teleport_id)?;
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
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_ACCEPT_TELEPORTATION;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            teleport_id: reader.read_varint()?,
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
    pub on_ground: bool,
}

impl ServerboundPacket for MovePlayerPosRot {
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_MOVE_PLAYER_POS_ROT;

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
            on_ground: reader.read_bool()?,
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

impl ServerboundPacket for MovePlayerPos {
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_MOVE_PLAYER_POS;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            x: f64::from_be_bytes(reader.read_const()?),
            y: f64::from_be_bytes(reader.read_const()?),
            z: f64::from_be_bytes(reader.read_const()?),
            on_ground: reader.read_bool()?,
        })
    }
}

#[derive(Debug)]
pub struct MovePlayerRot {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ServerboundPacket for MovePlayerRot {
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_MOVE_PLAYER_ROT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            yaw: f32::from_be_bytes(reader.read_const()?),
            pitch: f32::from_be_bytes(reader.read_const()?),
            on_ground: reader.read_bool()?,
        })
    }
}

#[derive(Debug)]
pub struct ClientTickEnd;

impl ServerboundPacket for ClientTickEnd {
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_CLIENT_TICK_END;

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
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_PLAYER_INPUT;

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
    const CLIENTBOUND_ID: i32 =
        generated::packet::play::CLIENTBOUND_MINECRAFT_SET_CHUNK_CACHE_CENTER;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_varint(self.chunk_x)?;
        writer.write_varint(self.chunk_z)?;
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
pub struct LevelChunkWithLight {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub heightmaps: NBT,
    pub data: Box<[u8]>,
    pub block_entities: Vec<BlockEntity>,
    // I have absolutely no clue on how the lighting information works right now.
    pub sky_light_mask: BitSet,
    pub block_light_mask: BitSet,
    pub empty_sky_light_mask: BitSet,
    pub empty_block_light_mask: BitSet,
    pub sky_lights_arrays: Vec<Vec<Vec<u8>>>,
    pub block_lights_arrays: Vec<Vec<Vec<u8>>>,
}

impl LevelChunkWithLight {
    pub fn generate_test(chunk_x: i32, chunk_z: i32, num_sections: usize) -> std::io::Result<Self> {
        Ok(Self {
            chunk_x,
            chunk_z,
            heightmaps: nbt_compound!(),
            data: {
                let mut writer = Vec::new();

                for i in 0..num_sections {
                    let block_id = if i == 0 { 1 } else { 0 };
                    // Num non-air blocks
                    writer.write_all(&if block_id != 0 { 4096i16 } else { 0i16 }.to_be_bytes())?;
                    // Blocks
                    writer.write_all(&to_paletted_data(&[block_id; 4096], 4..=8, 15)?)?;
                    // Biome
                    writer.write_all(&to_paletted_data(&[0; 64], 1..=3, 6)?)?;
                }

                writer.into_boxed_slice()
            },
            block_entities: Vec::new(),
            // Empty lighting data for now.
            sky_light_mask: BitSet::new(num_sections + 2),
            block_light_mask: BitSet::new(num_sections + 2),
            empty_sky_light_mask: BitSet::new(num_sections + 2),
            empty_block_light_mask: BitSet::new(num_sections + 2),
            sky_lights_arrays: Vec::new(),
            block_lights_arrays: Vec::new(),
        })
    }
}

impl ClientboundPacket for LevelChunkWithLight {
    const CLIENTBOUND_ID: i32 =
        generated::packet::play::CLIENTBOUND_MINECRAFT_LEVEL_CHUNK_WITH_LIGHT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_all(&self.chunk_x.to_be_bytes())?;
        writer.write_all(&self.chunk_z.to_be_bytes())?;
        writer.write_nbt(&self.heightmaps)?;
        writer.write_varint(self.data.len() as i32)?;
        writer.write_all(&self.data)?;
        writer.write_varint(self.block_entities.len() as i32)?;
        for block_entity in self.block_entities.iter() {
            writer.write_all(
                &(((block_entity.x & 0x0F) << 4) | (block_entity.z & 0x0F)).to_ne_bytes(),
            )?;
            writer.write_all(&block_entity.y.to_be_bytes())?;
            writer.write_varint(block_entity.r#type)?;
            writer.write_nbt(&block_entity.data)?;
        }
        // Skip lighting data for now.
        writer.write_varint(0)?;
        writer.write_varint(0)?;
        writer.write_varint(0)?;
        writer.write_varint(0)?;
        writer.write_varint(0)?;
        writer.write_varint(0)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ForgetLevelChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for ForgetLevelChunk {
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_FORGET_LEVEL_CHUNK;

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
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_PLAYER_ABILITIES;

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
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_PLAYER_ABILITIES;

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
    const SERVERBOUND_ID: i32 = generated::packet::play::SERVERBOUND_MINECRAFT_PLAYER_COMMAND;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            entity_id: reader.read_varint()?,
            action: PlayerCommandAction::try_from(reader.read_varint()?)?,
            jump_boost: reader.read_varint()?,
        })
    }
}

#[derive(Debug)]
pub struct SystemChat {
    pub content: TextComponent,
    pub overlay: bool,
}

impl ClientboundPacket for SystemChat {
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_SYSTEM_CHAT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_nbt(&self.content.to_nbt())?;
        writer.write_bool(self.overlay)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetActionBarText(pub TextComponent);

impl ClientboundPacket for SetActionBarText {
    const CLIENTBOUND_ID: i32 = generated::packet::play::CLIENTBOUND_MINECRAFT_SET_ACTION_BAR_TEXT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.write_nbt(&self.0.to_nbt())?;
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
    ClientTickEnd, ClientTickEnd;
    PlayerInput, PlayerInput;
    PlayerAbilities_Serverbound, PlayerAbilities;
    PlayerCommand, PlayerCommand;
);
