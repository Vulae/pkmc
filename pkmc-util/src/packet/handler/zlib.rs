use std::io::Write as _;

use crate::{
    packet::{ConnectionError, PacketDecoder as _, PacketEncoder as _},
    ReadExt,
};

/// Compresses packets beyond the specified threshold.
#[derive(Debug, Clone)]
pub struct ZlibPacketHandler {
    threshold: usize,
    compression_level: u32,
}

impl ZlibPacketHandler {
    /// compression_level panics if outside of 0..=9
    pub fn new(threshold: usize, compression_level: u32) -> Self {
        assert!(compression_level <= 9);
        Self {
            threshold,
            compression_level,
        }
    }

    pub(super) fn write(&self, raw: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        if raw.len() < self.threshold {
            let mut writer = Vec::new();
            writer.encode(0)?;
            writer.write_all(raw)?;
            Ok(writer.into_boxed_slice())
        } else {
            let mut compressed = flate2::write::ZlibEncoder::new(
                Vec::new(),
                flate2::Compression::new(self.compression_level),
            );
            compressed.write_all(raw)?;
            let compressed = compressed.flush_finish()?;

            let mut writer = Vec::new();
            writer.encode(raw.len() as i32)?;
            writer.write_all(&compressed)?;

            Ok(writer.into_boxed_slice())
        }
    }

    pub(super) fn read(&self, buf: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        let mut reader = std::io::Cursor::new(buf);
        match reader.decode::<i32>()? {
            0 => Ok(reader.read_all()?),
            _uncompressed_size => Ok(flate2::read::ZlibDecoder::new(reader).read_all()?),
        }
    }
}
