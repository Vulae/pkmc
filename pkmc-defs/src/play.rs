use std::io::{Read, Write};

use pkmc_nbt::{nbt_compound, NBT};
use pkmc_packet::{
    connection::{ClientboundPacket, PacketError, ServerboundPacket},
    to_paletted_container, BitSet, Position, ReadExtPacket, WriteExtPacket,
};
use pkmc_util::ReadExt;

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

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
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
        writer.write_bool(self.enforces_secure_chat)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum GameEvent {
    StartWaitingForLevelChunks,
}

impl ClientboundPacket for GameEvent {
    const CLIENTBOUND_ID: i32 = 0x22;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
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
    const CLIENTBOUND_ID: i32 = 0x26;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
        writer.write_all(&self.id.to_be_bytes())?;
        Ok(())
    }
}

impl ServerboundPacket for KeepAlive {
    const SERVERBOUND_ID: i32 = 0x18;

    fn packet_read(mut reader: impl Read) -> Result<Self, PacketError>
    where
        Self: Sized,
    {
        Ok(Self {
            id: i64::from_be_bytes(reader.read_const()?),
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

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
        writer.write_all(&self.x.unwrap_or(0.0).to_be_bytes())?;
        writer.write_all(&self.y.unwrap_or(0.0).to_be_bytes())?;
        writer.write_all(&self.z.unwrap_or(0.0).to_be_bytes())?;
        writer.write_all(&self.yaw.unwrap_or(0.0).to_be_bytes())?;
        writer.write_all(&self.pitch.unwrap_or(0.0).to_be_bytes())?;
        writer.write_all(
            &(if self.x.is_some() { 0x01u8 } else { 0 }
                | if self.y.is_some() { 0x02 } else { 0 }
                | if self.z.is_some() { 0x04 } else { 0 }
                | if self.yaw.is_some() { 0x08 } else { 0 }
                | if self.pitch.is_some() { 0x10 } else { 0 }
            // NOTE: This isn't an actual flag, Minecraft just checks if the flags are unset
            // to determine whether the teleport is absolute (=0x00) or relative (!=0x00)
                | if self.relative { 0x20 } else { 0 })
            .to_be_bytes(),
        )?;
        writer.write_varint(self.teleport_id)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConfirmTeleport {
    pub teleport_id: i32,
}

impl ServerboundPacket for ConfirmTeleport {
    const SERVERBOUND_ID: i32 = 0x00;

    fn packet_read(mut reader: impl Read) -> Result<Self, PacketError>
    where
        Self: Sized,
    {
        Ok(Self {
            teleport_id: reader.read_varint()?,
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

    fn packet_read(mut reader: impl Read) -> Result<Self, PacketError>
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
pub struct SetPlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

impl ServerboundPacket for SetPlayerPosition {
    const SERVERBOUND_ID: i32 = 0x1A;

    fn packet_read(mut reader: impl Read) -> Result<Self, PacketError>
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
pub struct SetPlayerRotation {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl ServerboundPacket for SetPlayerRotation {
    const SERVERBOUND_ID: i32 = 0x1C;

    fn packet_read(mut reader: impl Read) -> Result<Self, PacketError>
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
pub struct SetCenterChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for SetCenterChunk {
    const CLIENTBOUND_ID: i32 = 0x54;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
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

impl ChunkDataAndUpdateLight {
    pub fn generate_test(chunk_x: i32, chunk_z: i32, num_sections: usize) -> std::io::Result<Self> {
        Ok(Self {
            chunk_x,
            chunk_z,
            heightmaps: nbt_compound!(),
            data: {
                let mut writer = Vec::new();

                struct Section {
                    data: [i32; 4096],
                    air: [bool; 4096],
                }
                #[allow(dead_code)]
                impl Section {
                    pub fn new_empty() -> Self {
                        Self {
                            data: [0; 4096],
                            air: [false; 4096],
                        }
                    }

                    pub fn fill(&mut self, block: i32, air: bool) {
                        self.data.fill(block);
                        self.air.fill(air);
                    }

                    pub fn set(&mut self, x: u8, y: u8, z: u8, block: i32, air: bool) {
                        let ind = (((y as usize * 16) + z as usize) * 16) + x as usize;
                        self.data[ind] = block;
                        self.air[ind] = air;
                    }

                    pub fn write(&self, mut writer: impl Write) -> std::io::Result<()> {
                        writer.write_all(
                            &(self.air.iter().filter(|v| !*v).count() as i16).to_be_bytes(),
                        )?;
                        writer.write_all(&to_paletted_container(&self.data, 4, 8)?)?;
                        Ok(())
                    }
                }

                for _ in 0..num_sections {
                    let mut section = Section::new_empty();
                    section.fill(1, false);
                    section.write(&mut writer)?;
                    // Biome??
                    writer.write_all(&to_paletted_container(&[0; 64], 1, 3)?)?;
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

impl ClientboundPacket for ChunkDataAndUpdateLight {
    const CLIENTBOUND_ID: i32 = 0x27;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
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
pub struct UnloadChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl ClientboundPacket for UnloadChunk {
    const CLIENTBOUND_ID: i32 = 0x21;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
        writer.write_all(&self.chunk_z.to_be_bytes())?;
        writer.write_all(&self.chunk_x.to_be_bytes())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct PlayerAbilities {
    pub flags: u8,
    pub flying_speed: f32,
    pub field_of_view_modifier: f32,
}

impl ClientboundPacket for PlayerAbilities {
    const CLIENTBOUND_ID: i32 = 0x38;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), PacketError> {
        writer.write_all(&self.flags.to_be_bytes())?;
        writer.write_all(&self.flying_speed.to_be_bytes())?;
        writer.write_all(&self.field_of_view_modifier.to_be_bytes())?;
        Ok(())
    }
}
