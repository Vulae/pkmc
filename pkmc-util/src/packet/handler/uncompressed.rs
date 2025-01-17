use std::{
    collections::VecDeque,
    io::{Read as _, Write},
};

use crate::packet::{
    try_read_varint_ret_bytes, varint_size, ConnectionError, RawPacket, WriteExtPacket as _,
};

#[derive(Debug, Clone)]
pub struct UncompressedPacketHandler;

impl UncompressedPacketHandler {
    pub fn write(&self, packet: &RawPacket, mut stream: impl Write) -> Result<(), ConnectionError> {
        stream.write_varint(varint_size(packet.id) + (packet.data.len() as i32))?;
        stream.write_varint(packet.id)?;
        stream.write_all(&packet.data)?;
        stream.flush()?;
        Ok(())
    }

    pub fn read(&self, buf: &mut VecDeque<u8>) -> Result<Option<RawPacket>, ConnectionError> {
        // I'm so sorry.
        let Some((length_bytes, length)) = try_read_varint_ret_bytes(buf.make_contiguous())? else {
            return Ok(None);
        };
        if buf.len() < length as usize {
            return Ok(None);
        }
        buf.drain(0..length_bytes);
        let Some((id_bytes, id)) = try_read_varint_ret_bytes(buf.make_contiguous())? else {
            todo!();
        };
        buf.drain(0..id_bytes);
        let mut data = vec![0u8; (length as usize) - id_bytes];
        buf.read_exact(&mut data)?;
        Ok(Some(RawPacket {
            id,
            data: data.into_boxed_slice(),
        }))
    }
}
