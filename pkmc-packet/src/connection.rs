use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::TcpStream,
};

use pkmc_util::ReadExt;
use thiserror::Error;

use crate::{
    reader::try_read_varint_ret_bytes,
    writer::{varint_size, WriteExtPacket},
};

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("{0:?}")]
    IoError(#[from] std::io::Error),
    #[error("{0:?}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
    #[error("Unsupported packet {0}: {1:#X}")]
    UnsupportedPacket(String, i32),
}

pub trait ServerboundPacket {
    const SERVERBOUND_ID: i32;

    fn serverbound_id(&self) -> i32 {
        Self::SERVERBOUND_ID
    }

    fn packet_read(reader: impl Read) -> Result<Self, PacketError>
    where
        Self: Sized;
}

pub trait ClientboundPacket {
    const CLIENTBOUND_ID: i32;

    fn clientbound_id(&self) -> i32 {
        Self::CLIENTBOUND_ID
    }

    fn packet_write(&self, writer: impl Write) -> Result<(), PacketError>;

    fn raw_packet(&self) -> Result<RawPacket, PacketError> {
        let mut raw_data = Vec::new();
        self.packet_write(&mut raw_data)?;
        Ok(RawPacket {
            id: self.clientbound_id(),
            data: raw_data.into_boxed_slice(),
        })
    }
}

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error("{0:?}")]
    PacketError(#[from] PacketError),
    #[error("{0:?}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct UncompressedStreamHandler;

impl UncompressedStreamHandler {
    fn send(&mut self, mut writer: impl Write, packet: &RawPacket) -> Result<(), ConnectionError> {
        writer.write_varint(varint_size(packet.id) + (packet.data.len() as i32))?;
        writer.write_varint(packet.id)?;
        writer.write_all(&packet.data)?;
        writer.flush()?;
        Ok(())
    }

    fn recieve(&mut self, buf: &mut VecDeque<u8>) -> Result<Option<RawPacket>, ConnectionError> {
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

#[derive(Debug)]
pub struct ZlibStreamHandler {
    threshold: usize,
    compression_level: u32,
}

impl ZlibStreamHandler {
    /// compression_level 1..=9
    pub fn new(threshold: usize, compression_level: u32) -> Self {
        // TODO: Error handling
        if compression_level == 0 {
            panic!("DO NOT USE COMPRESSION LEVEL 0! JUST DISABLE COMPRESSION INSTEAD.");
        }
        if compression_level > 9 {
            panic!("INVALID COMPRESSION LEVEL");
        }
        Self {
            threshold,
            compression_level,
        }
    }

    fn send(&mut self, mut writer: impl Write, packet: &RawPacket) -> Result<(), ConnectionError> {
        if packet.data.len() < self.threshold {
            writer.write_varint(
                varint_size(packet.id) + varint_size(0) + (packet.data.len() as i32),
            )?;
            writer.write_varint(0)?;
            writer.write_varint(packet.id)?;
            writer.write_all(&packet.data)?;
        } else {
            let mut encoder = flate2::write::ZlibEncoder::new(
                Vec::new(),
                flate2::Compression::new(self.compression_level),
            );
            encoder.write_varint(packet.id)?;
            encoder.write_all(&packet.data)?;
            let compressed = encoder.flush_finish()?;
            writer.write_varint(varint_size(packet.data.len() as i32) + compressed.len() as i32)?;
            writer.write_varint(varint_size(packet.id) + packet.data.len() as i32)?;
            writer.write_all(&compressed)?;
        }
        writer.flush()?;
        Ok(())
    }

    fn recieve(&mut self, buf: &mut VecDeque<u8>) -> Result<Option<RawPacket>, ConnectionError> {
        // I'm even more sorry.
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

#[derive(Debug)]
pub enum StreamHandler {
    Uncompressed(UncompressedStreamHandler),
    Zlib(ZlibStreamHandler),
}

impl StreamHandler {
    fn send(&mut self, writer: impl Write, packet: &RawPacket) -> Result<(), ConnectionError> {
        match self {
            StreamHandler::Uncompressed(handler) => handler.send(writer, packet),
            StreamHandler::Zlib(handler) => handler.send(writer, packet),
        }
    }

    fn recieve(&mut self, buf: &mut VecDeque<u8>) -> Result<Option<RawPacket>, ConnectionError> {
        match self {
            StreamHandler::Uncompressed(handler) => handler.recieve(buf),
            StreamHandler::Zlib(handler) => handler.recieve(buf),
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
    closed: bool,
    bytes: VecDeque<u8>,
    pub handler: StreamHandler,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RawPacket {
    pub id: i32,
    pub data: Box<[u8]>,
}

impl RawPacket {
    pub fn new(id: i32, data: Box<[u8]>) -> Self {
        Self { id, data }
    }
}

impl Connection {
    pub fn new(stream: TcpStream) -> Result<Self, ConnectionError> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            closed: false,
            bytes: VecDeque::new(),
            handler: StreamHandler::Uncompressed(UncompressedStreamHandler),
        })
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    fn recieve_bytes(&mut self) -> Result<(), std::io::Error> {
        let mut buf = [0u8; 1024];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => {
                    self.closed = true;
                    return Ok(());
                }
                Ok(n) => {
                    self.bytes.extend(&buf[..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::BrokenPipe => {
                    self.closed = true;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub fn send(&mut self, packet: impl ClientboundPacket) -> Result<(), ConnectionError> {
        self.handler.send(&mut self.stream, &packet.raw_packet()?)?;
        Ok(())
    }

    pub fn recieve(&mut self) -> Result<Option<RawPacket>, ConnectionError> {
        self.recieve_bytes()?;
        self.handler.recieve(&mut self.bytes)
    }
}

// I gave up on this macro trying to get it to work with rustfmt
//#[macro_export]
//macro_rules! match_packet_parse {
//    ($packet:expr; $fallback_pat:pat => $fallback_block:stmt, $($parser:ty, $name:pat => $block:stmt,)*) => {
//        if let Some(packet) = $packet {
//            let mut reader = $crate::packet::reader::PacketReader::new(std::io::Cursor::new(packet.data.as_ref()));
//            match packet {
//                $(
//                    $crate::connection::RawPacket { id: <$parser>::SERVERBOUND_ID, .. } => {
//                        let $name = <$parser>::packet_read(&mut reader)?;
//                        $block
//                    },)*
//                $fallback_pat => { $fallback_block },
//            }
//        }
//    };
//}

#[macro_export]
macro_rules! create_packet_enum {
    ($enum_name:ident; $($type:ty, $name:ident;)*) => {
        #[allow(unused)]
        enum $enum_name {
            $(
                $name($type),
            )*
        }

        impl TryFrom<$crate::connection::RawPacket> for $enum_name {
            type Error = $crate::connection::PacketError;

            fn try_from(value: $crate::connection::RawPacket) -> std::result::Result<Self, Self::Error> {
                use $crate::connection::ServerboundPacket as _;
                let mut reader = std::io::Cursor::new(value.data);
                match value.id {
                    $(
                        <$type>::SERVERBOUND_ID => Ok(Self::$name(<$type>::packet_read(&mut reader)?)),
                    )*
                    _ => Err(Self::Error::UnsupportedPacket(stringify!($enum_name).to_owned(), value.id)),
                }
            }
        }
    }
}
