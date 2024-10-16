use super::{writer::PacketWriter, Packet};
use anyhow::Result;

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

impl Packet for SynchronizePlayerPosition {
    const ID: i32 = 64;

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

impl Packet for ConfirmTeleport {
    const ID: i32 = 0;

    fn packet_read(reader: &mut super::reader::PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            teleport_id: reader.read_var_int()?,
        })
    }
}

#[derive(Debug)]
pub struct GameEvent {
    pub event: u8,
    pub value: f32,
}

impl Packet for GameEvent {
    const ID: i32 = 34;

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        writer.write_unsigned_byte(self.event)?;
        writer.write_float(self.value)?;
        Ok(())
    }
}
