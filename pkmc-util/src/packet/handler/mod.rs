pub mod uncompressed;
pub mod zlib;

use std::{collections::VecDeque, fmt::Debug, io::Write};

use super::{ConnectionError, RawPacket};

pub use uncompressed::*;
pub use zlib::*;

#[derive(Debug, Clone)]
pub enum PacketHandler {
    Uncompressed(UncompressedPacketHandler),
    Zlib(ZlibPacketHandler),
}

impl PacketHandler {
    pub fn write(&self, packet: &RawPacket, stream: impl Write) -> Result<(), ConnectionError> {
        match self {
            PacketHandler::Uncompressed(uncompressed_packet_handler) => {
                uncompressed_packet_handler.write(packet, stream)
            }
            PacketHandler::Zlib(zlib_packet_handler) => zlib_packet_handler.write(packet, stream),
        }
    }

    pub fn read(&self, buf: &mut VecDeque<u8>) -> Result<Option<RawPacket>, ConnectionError> {
        match self {
            PacketHandler::Uncompressed(uncompressed_packet_handler) => {
                uncompressed_packet_handler.read(buf)
            }
            PacketHandler::Zlib(zlib_packet_handler) => zlib_packet_handler.read(buf),
        }
    }
}
