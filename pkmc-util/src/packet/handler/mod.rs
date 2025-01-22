pub mod uncompressed;
pub mod zlib;

use std::fmt::Debug;

use super::ConnectionError;

pub use uncompressed::*;
pub use zlib::*;

#[derive(Debug, Clone)]
pub enum PacketHandler {
    Uncompressed(UncompressedPacketHandler),
    Zlib(ZlibPacketHandler),
}

impl PacketHandler {
    pub fn write(&self, raw: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        match self {
            PacketHandler::Uncompressed(uncompressed_packet_handler) => {
                uncompressed_packet_handler.write(raw)
            }
            PacketHandler::Zlib(zlib_packet_handler) => zlib_packet_handler.write(raw),
        }
    }

    pub fn read(&self, buf: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        match self {
            PacketHandler::Uncompressed(uncompressed_packet_handler) => {
                uncompressed_packet_handler.read(buf)
            }
            PacketHandler::Zlib(zlib_packet_handler) => zlib_packet_handler.read(buf),
        }
    }
}
