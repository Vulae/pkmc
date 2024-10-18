use crate::{
    connection::{ClientboundPacket, ServerboundPacket},
    nbt::NBT,
};

use super::{reader::PacketReader, writer::PacketWriter, BitSet, Position};
use anyhow::Result;

pub struct LoginPlay {
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
    pub enforces_secure_chat: bool,
}

impl ClientboundPacket for LoginPlay {
    const CLIENTBOUND_ID: i32 = 0x2B;

    fn packet_write(&self, writer: &mut super::writer::PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_int(self.entity_id)?;
        writer.write_boolean(self.is_hardcore)?;
        writer.write_var_int(self.dimensions.len() as i32)?;
        for dimension in self.dimensions.iter() {
            writer.write_string(dimension)?;
        }
        writer.write_var_int(self.max_players)?;
        writer.write_var_int(self.view_distance)?;
        writer.write_var_int(self.simulation_distance)?;
        writer.write_boolean(self.reduced_debug_info)?;
        writer.write_boolean(self.enable_respawn_screen)?;
        writer.write_boolean(self.do_limited_crafting)?;
        writer.write_var_int(self.dimension_type)?;
        writer.write_string(&self.dimension_name)?;
        writer.write_long(self.hashed_seed)?;
        writer.write_unsigned_byte(self.game_mode)?;
        writer.write_signed_byte(self.previous_game_mode)?;
        writer.write_boolean(self.is_debug)?;
        writer.write_boolean(self.is_flat)?;
        if let Some(death) = &self.death {
            writer.write_boolean(true)?;
            writer.write_string(&death.0)?;
            writer.write_position(death.1)?;
        } else {
            writer.write_boolean(false)?;
        }
        writer.write_var_int(self.portal_cooldown)?;
        writer.write_boolean(self.enforces_secure_chat)?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum GameEvent {
    StartWaitingForLevelChunks,
}

impl ClientboundPacket for GameEvent {
    const CLIENTBOUND_ID: i32 = 0x22;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        match self {
            GameEvent::StartWaitingForLevelChunks => {
                writer.write_unsigned_byte(13)?;
                writer.write_float(0.0)?;
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
    const CLIENTBOUND_ID: i32 = 0x26;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_long(self.id)?;
        Ok(())
    }
}

impl ServerboundPacket for KeepAlive {
    const SERVERBOUND_ID: i32 = 0x18;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            id: reader.read_long()?,
        })
    }
}

#[derive(Debug)]
pub struct SynchronizePlayerPosition {
    pub relative: bool,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub yaw: Option<f32>,
    pub pitch: Option<f32>,
    pub teleport_id: i32,
}

impl ClientboundPacket for SynchronizePlayerPosition {
    const CLIENTBOUND_ID: i32 = 0x40;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_double(self.x.unwrap_or(0.0))?;
        writer.write_double(self.y.unwrap_or(0.0))?;
        writer.write_double(self.z.unwrap_or(0.0))?;
        writer.write_float(self.yaw.unwrap_or(0.0))?;
        writer.write_float(self.pitch.unwrap_or(0.0))?;
        writer.write_unsigned_byte(
            if self.x.is_some() { 0x01 } else { 0 }
                | if self.y.is_some() { 0x02 } else { 0 }
                | if self.z.is_some() { 0x04 } else { 0 }
                | if self.yaw.is_some() { 0x08 } else { 0 }
                | if self.pitch.is_some() { 0x10 } else { 0 }
            // NOTE: This isn't an actual flag, Minecraft just checks if the flags are unset
            // to determine whether the teleport is absolute (=0x00) or relative (!=0x00)
                | if self.relative { 0x20 } else { 0 },
        )?;
        writer.write_var_int(self.teleport_id)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConfirmTeleport {
    pub teleport_id: i32,
}

impl ServerboundPacket for ConfirmTeleport {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            teleport_id: reader.read_var_int()?,
        })
    }
}

#[derive(Debug)]
pub struct SetPlayerPositionAndRotation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ServerboundPacket for SetPlayerPositionAndRotation {
    const SERVERBOUND_ID: i32 = 0x1B;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            x: reader.read_double()?,
            y: reader.read_double()?,
            z: reader.read_double()?,
            yaw: reader.read_float()?,
            pitch: reader.read_float()?,
            on_ground: reader.read_boolean()?,
        })
    }
}

#[derive(Debug)]
pub struct SetPlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

impl ServerboundPacket for SetPlayerPosition {
    const SERVERBOUND_ID: i32 = 0x1A;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            x: reader.read_double()?,
            y: reader.read_double()?,
            z: reader.read_double()?,
            on_ground: reader.read_boolean()?,
        })
    }
}

#[derive(Debug)]
pub struct SetPlayerRotation {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ServerboundPacket for SetPlayerRotation {
    const SERVERBOUND_ID: i32 = 0x1C;

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            yaw: reader.read_float()?,
            pitch: reader.read_float()?,
            on_ground: reader.read_boolean()?,
        })
    }
}

#[derive(Debug)]
pub struct SetCenterChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for SetCenterChunk {
    const CLIENTBOUND_ID: i32 = 0x54;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_var_int(self.chunk_x)?;
        writer.write_var_int(self.chunk_z)?;
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
pub struct ChunkDataAndUpdateLight {
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

impl ClientboundPacket for ChunkDataAndUpdateLight {
    const CLIENTBOUND_ID: i32 = 0x27;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_int(self.chunk_x)?;
        writer.write_int(self.chunk_z)?;
        writer.write_nbt(&self.heightmaps)?;
        writer.write_var_int(self.data.len() as i32)?;
        writer.write_buf(&self.data)?;
        writer.write_var_int(self.block_entities.len() as i32)?;
        for block_entity in self.block_entities.iter() {
            writer.write_unsigned_byte(((block_entity.x & 0x0F) << 4) | (block_entity.z & 0x0F))?;
            writer.write_short(block_entity.y)?;
            writer.write_var_int(block_entity.r#type)?;
            writer.write_nbt(&block_entity.data)?;
        }
        // Skip lighting data for now.
        writer.write_var_int(0)?;
        writer.write_var_int(0)?;
        writer.write_var_int(0)?;
        writer.write_var_int(0)?;
        writer.write_var_int(0)?;
        writer.write_var_int(0)?;
        Ok(())
    }
}
