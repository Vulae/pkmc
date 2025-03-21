use std::{
    fmt,
    io::{Read, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    packet::{PacketDecodable, PacketEncodable},
    ReadExt,
};

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct UUID(pub [u8; 16]);

impl UUID {
    pub fn new_v7() -> Self {
        let time_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut value: u128 = rand::random();
        value &= !0xFFFFFFFF_FFFF_0000_0000_000000000000;
        value |= time_since_epoch.as_millis() << 80;
        value &= !0x00000000_0000_F000_F000_000000000000;
        value |= 0x00000000_0000_7000_B000_000000000000;
        Self(value.to_le_bytes())
    }
}

impl fmt::Display for UUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format each part of the UUID
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5],
            self.0[6], self.0[7],
            self.0[8], self.0[9],
            self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }
}

impl PacketEncodable for &UUID {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&self.0)
    }
}

impl PacketDecodable for UUID {
    fn packet_decode(mut reader: impl Read) -> std::io::Result<Self> {
        Ok(UUID(reader.read_const()?))
    }
}
