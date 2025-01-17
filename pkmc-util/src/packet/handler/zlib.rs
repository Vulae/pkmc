use std::{
    collections::VecDeque,
    io::{Read as _, Write},
};

use crate::{
    packet::{
        try_read_varint_ret_bytes, varint_size, ConnectionError, RawPacket, WriteExtPacket as _,
    },
    ReadExt as _,
};

#[derive(Debug, Clone)]
pub struct ZlibPacketHandler {
    threshold: usize,
    compression_level: u32,
}

impl ZlibPacketHandler {
    pub fn new(threshold: usize, compression_level: u32) -> Self {
        assert!(compression_level <= 9);
        Self {
            threshold,
            compression_level,
        }
    }

    pub fn write(&self, packet: &RawPacket, mut stream: impl Write) -> Result<(), ConnectionError> {
        let total_uncompressed_size =
            varint_size(0) as usize + varint_size(packet.id) as usize + packet.data.len();
        if total_uncompressed_size < self.threshold {
            stream.write_varint(total_uncompressed_size as i32)?;
            stream.write_varint(0)?;
            stream.write_varint(packet.id)?;
            stream.write_all(&packet.data)?;
        } else {
            let mut encoder = flate2::write::ZlibEncoder::new(
                Vec::new(),
                flate2::Compression::new(self.compression_level),
            );
            encoder.write_varint(packet.id)?;
            encoder.write_all(&packet.data)?;
            let compressed = encoder.flush_finish()?;
            stream.write_varint(varint_size(packet.data.len() as i32) + compressed.len() as i32)?;
            stream.write_varint(varint_size(packet.id) + packet.data.len() as i32)?;
            stream.write_all(&compressed)?;
        }
        stream.flush()?;
        Ok(())
    }

    pub fn read(&self, buf: &mut VecDeque<u8>) -> Result<Option<RawPacket>, ConnectionError> {
        // Please forgive me ;-;
        let Some((length_bytes, length)) = try_read_varint_ret_bytes(buf.make_contiguous())? else {
            return Ok(None);
        };
        if buf.len() < length as usize {
            return Ok(None);
        }
        buf.drain(0..length_bytes);
        let Some((uncompressed_length_bytes, uncompressed_length)) =
            try_read_varint_ret_bytes(buf.make_contiguous())?
        else {
            todo!()
        };
        buf.drain(0..uncompressed_length_bytes);
        let (id, data) = if uncompressed_length == 0 {
            let Some((id_bytes, id)) = try_read_varint_ret_bytes(buf.make_contiguous())? else {
                todo!();
            };
            buf.drain(0..id_bytes);
            let mut data = vec![0u8; (length as usize) - uncompressed_length_bytes - id_bytes];
            buf.read_exact(&mut data)?;
            (id, data)
        } else {
            let mut compressed = vec![0u8; (length as usize) - uncompressed_length_bytes];
            buf.read_exact(&mut compressed)?;
            let uncompressed =
                flate2::read::ZlibDecoder::new(std::io::Cursor::new(compressed)).read_all()?;
            let Some((id_bytes, id)) = try_read_varint_ret_bytes(&uncompressed)? else {
                todo!();
            };
            (id, uncompressed[id_bytes..].to_vec())
        };
        Ok(Some(RawPacket {
            id,
            data: data.into_boxed_slice(),
        }))
    }
}
