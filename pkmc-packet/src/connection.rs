use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::TcpStream,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
};

use pkmc_util::ReadExt;
use thiserror::Error;

use crate::{
    reader::try_read_varint_ret_bytes,
    writer::{varint_size, WriteExtPacket},
    ClientboundPacket, RawPacket,
};

#[derive(Error, Debug)]
pub enum ConnectionError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Other(Box<dyn std::error::Error + Send + Sync>),
    #[error("Unsupported packet {0}: {1:#X}")]
    UnsupportedPacket(String, i32),
    #[error("Invalid raw packet ID for parser (expected: {0}, found: {1})")]
    InvalidRawPacketIDForParser(i32, i32),
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
struct InnerConnection {
    stream: TcpStream,
    rx: Receiver<RawPacket>,
    bytes: VecDeque<u8>,
    handler: StreamHandler,
    closed: bool,
}

impl InnerConnection {
    fn recieve_bytes(&mut self) -> Result<(), std::io::Error> {
        let mut buf = [0u8; 1024];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => {
                    self.closed = true;
                    break;
                }
                Ok(n) => {
                    self.bytes.extend(&buf[..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(ref e)
                    if e.kind() == io::ErrorKind::BrokenPipe
                        || e.kind() == io::ErrorKind::UnexpectedEof =>
                {
                    self.closed = true;
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn send(&mut self, packet: impl ClientboundPacket) -> Result<(), ConnectionError> {
        self.handler.send(&mut self.stream, &packet.raw_packet()?)?;
        Ok(())
    }

    fn try_send_all(&mut self) -> Result<(), ConnectionError> {
        loop {
            match self.rx.try_recv() {
                Ok(raw_packet) => self.handler.send(&mut self.stream, &raw_packet)?,
                Err(TryRecvError::Empty) => break,
                Err(_err) => unreachable!(),
            }
        }
        Ok(())
    }

    fn recieve(&mut self) -> Result<Option<RawPacket>, ConnectionError> {
        self.recieve_bytes()?;
        if self.closed {
            return Ok(None);
        }
        self.handler.recieve(&mut self.bytes)
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    inner: Arc<Mutex<Option<InnerConnection>>>,
    tx: Option<Sender<RawPacket>>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Result<Self, ConnectionError> {
        stream.set_nonblocking(true)?;
        let (tx, rx) = mpsc::channel::<RawPacket>();
        Ok(Self {
            inner: Arc::new(Mutex::new(Some(InnerConnection {
                stream,
                rx,
                bytes: VecDeque::new(),
                handler: StreamHandler::Uncompressed(UncompressedStreamHandler),
                closed: false,
            }))),
            tx: Some(tx),
        })
    }

    fn update_closed_state(&mut self) {
        self.inner.lock().unwrap().take_if(|inner| inner.closed);
        if self.inner.lock().unwrap().is_none() {
            self.tx = None;
        }
    }

    pub fn close(&mut self) -> Result<(), ConnectionError> {
        if let Some(inner) = self.inner.lock().unwrap().take() {
            inner.stream.shutdown(std::net::Shutdown::Both)?;
        }
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map(|inner| inner.closed)
            .unwrap_or(true)
    }

    pub fn set_handler(&mut self, handler: StreamHandler) {
        self.update_closed_state();
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.handler = handler;
        }
    }

    pub fn send_async(&mut self, packet: impl ClientboundPacket) -> Result<(), ConnectionError> {
        if let Some(tx) = &self.tx {
            if let Err(_err) = tx.send(packet.raw_packet()?) {
                unreachable!();
            }
        }
        Ok(())
    }

    pub fn update_async(&mut self) -> Result<(), ConnectionError> {
        self.update_closed_state();
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.try_send_all()?;
        }
        Ok(())
    }

    pub fn send(&mut self, packet: impl ClientboundPacket) -> Result<(), ConnectionError> {
        self.update_closed_state();
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.send(packet)?;
        }
        Ok(())
    }

    pub fn recieve(&mut self) -> Result<Option<RawPacket>, ConnectionError> {
        self.update_closed_state();
        if let Some(inner) = self.inner.lock().unwrap().as_mut() {
            inner.recieve()
        } else {
            Ok(None)
        }
    }

    pub fn recieve_into<T>(&mut self) -> Result<Option<T>, ConnectionError>
    where
        T: TryFrom<RawPacket, Error = ConnectionError>,
    {
        self.recieve().map(|i| i.map(T::try_from).transpose())?
    }
}

#[macro_export]
macro_rules! serverbound_packet_enum {
    ($enum_vis:vis $enum_name:ident; $($type:ty, $name:ident;)*) => {
        #[allow(unused)]
        $enum_vis enum $enum_name {
            $(
                $name($type),
            )*
        }

        impl TryFrom<$crate::packet::RawPacket> for $enum_name {
            type Error = $crate::connection::ConnectionError;

            fn try_from(value: $crate::packet::RawPacket) -> std::result::Result<Self, Self::Error> {
                use $crate::packet::ServerboundPacket as _;
                let mut reader = std::io::Cursor::new(&value.data);
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
