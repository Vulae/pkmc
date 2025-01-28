use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use rand::Rng;

#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct UUID(pub [u8; 16]);

impl UUID {
    pub fn new_v7() -> Self {
        let time_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut value: u128 = rand::thread_rng().gen();
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
