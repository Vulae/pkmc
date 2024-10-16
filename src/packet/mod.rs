pub mod login;
pub mod play;
pub mod reader;
pub mod server_list;
pub mod writer;

use std::io::Cursor;

use anyhow::Result;
use reader::PacketReader;
use writer::PacketWriter;

/// WARNING: Packet::packet_read & Packet::packet_write have default implementations that panic!
pub trait Packet {
    const ID: i32;

    fn id(&self) -> i32 {
        Self::ID
    }

    #[allow(unused_variables)]
    fn packet_read(reader: &mut PacketReader<Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized,
    {
        unreachable!()
    }

    #[allow(unused_variables)]
    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()> {
        unreachable!()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i16,
    pub z: i32,
}
