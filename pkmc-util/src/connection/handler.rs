use std::{fmt::Debug, io::Write as _};

use thiserror::Error;

use crate::ReadExt as _;

use super::{PacketDecoder as _, PacketEncoder as _};

#[derive(Debug, Error)]
pub enum PacketHandlerError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Zlib compression must be inside range 0..=9, but got {0}")]
    InvalidZlibCompressionLevel(u32),
}

/// Handler for reading & writing packets for [`super::Connection`] & [`super::ConnectionSender`].
#[derive(Debug, Clone)]
pub enum PacketHandler {
    Uncompressed,
    Zlib {
        threshold: usize,
        /// MUST be in range 0..=9
        compression_level: u32,
    },
}

impl PacketHandler {
    pub fn new_zlib(threshold: usize, compression_level: u32) -> Result<Self, PacketHandlerError> {
        if compression_level > 9 {
            return Err(PacketHandlerError::InvalidZlibCompressionLevel(
                compression_level,
            ));
        }
        Ok(Self::Zlib {
            threshold,
            compression_level,
        })
    }

    pub(crate) fn write(&self, raw: &[u8]) -> Result<Box<[u8]>, PacketHandlerError> {
        match self {
            PacketHandler::Uncompressed => Ok(raw.to_owned().into_boxed_slice()),
            PacketHandler::Zlib {
                threshold,
                compression_level,
            } => {
                if raw.len() < *threshold {
                    let mut writer = Vec::new();
                    writer.encode(0)?;
                    writer.write_all(raw)?;
                    Ok(writer.into_boxed_slice())
                } else {
                    let mut compressed = flate2::write::ZlibEncoder::new(
                        Vec::new(),
                        flate2::Compression::new(*compression_level),
                    );
                    compressed.write_all(raw)?;
                    let compressed = compressed.flush_finish()?;

                    let mut writer = Vec::new();
                    writer.encode(raw.len() as i32)?;
                    writer.write_all(&compressed)?;

                    Ok(writer.into_boxed_slice())
                }
            }
        }
    }

    pub(crate) fn read(&self, buf: &[u8]) -> Result<Box<[u8]>, PacketHandlerError> {
        match self {
            PacketHandler::Uncompressed => Ok(buf.to_owned().into_boxed_slice()),
            PacketHandler::Zlib { .. } => {
                let mut reader = std::io::Cursor::new(buf);
                match reader.decode::<i32>()? {
                    0 => Ok(reader.read_all()?),
                    _uncompressed_size => Ok(flate2::read::ZlibDecoder::new(reader).read_all()?),
                }
            }
        }
    }
}
