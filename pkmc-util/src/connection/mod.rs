mod codec;
mod encryption;
mod handler;
mod packet;
pub mod paletted_container;
pub mod varint;

use encryption::ConnectionEncryptionError;
use handler::PacketHandlerError;
use std::{
    collections::VecDeque,
    io::{Read as _, Write as _},
    net::TcpStream,
    sync::{Arc, Mutex},
};
use thiserror::Error;

pub use codec::*;
pub use encryption::ConnectionEncryption;
pub use handler::PacketHandler;
pub use packet::*;
use varint::try_read_varint_ret_bytes;

use crate::ReadExt as _;

const PACKET_RECIEVE_BUFFER_SIZE: usize = 1024;

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
    #[error(transparent)]
    HandlerError(#[from] PacketHandlerError),
    #[error(transparent)]
    EncryptionError(#[from] ConnectionEncryptionError),
}

#[derive(Debug)]
struct ConnectionInner {
    stream: Option<TcpStream>,
    encryption: ConnectionEncryption,
    handler: PacketHandler,
}

/// Handling sending packets from a [`TcpStream`].
#[derive(Debug, Clone)]
pub struct ConnectionSender {
    inner: Arc<Mutex<ConnectionInner>>,
}

impl ConnectionSender {
    /// If the connection is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.lock().unwrap().stream.is_none()
    }

    /// Encode & send a packet.
    pub fn send(&self, packet: &impl ClientboundPacket) -> Result<(), ConnectionError> {
        let raw: RawPacket = packet.raw_packet()?;
        let bytes = raw.into_bytes();

        let handler = self.inner.lock().unwrap().handler.clone();

        let encoded = handler.write(&bytes)?;

        let mut with_size = Vec::new();
        with_size.encode(encoded.len() as i32)?;
        with_size.write_all(&encoded)?;

        let mut inner = self.inner.lock().unwrap();

        let encrypted = {
            inner.encryption.encrypt(&mut with_size)?;
            with_size
        };

        let Some(stream) = inner.stream.as_mut() else {
            return Ok(());
        };
        match stream.write_all(&encrypted) {
            Err(err)
                if err.kind() == std::io::ErrorKind::BrokenPipe
                    || err.kind() == std::io::ErrorKind::ConnectionReset =>
            {
                inner.stream = None
            }
            v => v?,
        }
        Ok(())
    }
}

/// Handling recieving & sending packets from a [`TcpStream`].
/// [`Connection`] is non-blocking.
///
/// [`Connection`] may be used to create any number of [`ConnectionSenders`], of which can only send
/// packets.
#[derive(Debug)]
pub struct Connection {
    inner: Arc<Mutex<ConnectionInner>>,
    bytes: VecDeque<u8>,
}

impl Connection {
    /// Create a new connection from TcpStream.
    pub fn new(stream: TcpStream) -> Result<Self, ConnectionError> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(ConnectionInner {
                stream: Some(stream),
                encryption: ConnectionEncryption::Unencrypted,
                handler: PacketHandler::Uncompressed,
            })),
            bytes: VecDeque::new(),
        })
    }

    /// Create a new [`ConnectionSender`] from [`Connection`]
    pub fn sender(&self) -> ConnectionSender {
        ConnectionSender {
            inner: self.inner.clone(),
        }
    }

    pub fn set_connection_encryption(&self, encryption: ConnectionEncryption) {
        self.inner.lock().unwrap().encryption = encryption;
    }

    /// Set packet handler to use, see [`PacketHandler`]
    pub fn set_packet_handler(&self, handler: PacketHandler) {
        self.inner.lock().unwrap().handler = handler;
    }

    /// If the connection is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.lock().unwrap().stream.is_none()
    }

    /// Close the connection.
    pub fn close(&self) {
        self.inner.lock().unwrap().stream = None;
    }

    /// Encode & send a packet.
    pub fn send(&self, packet: &impl ClientboundPacket) -> Result<(), ConnectionError> {
        self.sender().send(packet)
    }

    fn recieve_bytes(&mut self) -> Result<(), ConnectionError> {
        let mut buf = [0u8; PACKET_RECIEVE_BUFFER_SIZE];
        let mut inner = self.inner.lock().unwrap();
        loop {
            match if let Some(stream) = inner.stream.as_mut() {
                stream
            } else {
                return Ok(());
            }
            .read(&mut buf)
            {
                Ok(0) => {
                    inner.stream = None;
                    break;
                }
                Ok(n) => {
                    // Clippy trippin
                    #[allow(clippy::unnecessary_mut_passed)]
                    inner.encryption.decrypt(&mut buf[..n])?;
                    self.bytes.extend(&buf[..n])
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(err)
                    if err.kind() == std::io::ErrorKind::BrokenPipe
                        || err.kind() == std::io::ErrorKind::UnexpectedEof
                        || err.kind() == std::io::ErrorKind::ConnectionReset =>
                {
                    inner.stream = None;
                    break;
                }
                Err(err) => return Err(err)?,
            }
        }
        Ok(())
    }

    /// Recieve a raw packet if available.
    pub fn recieve(&mut self) -> Result<Option<RawPacket>, ConnectionError> {
        self.recieve_bytes()?;

        let Some((size_bytes, size)) = try_read_varint_ret_bytes(self.bytes.make_contiguous())?
        else {
            return Ok(None);
        };

        if self.bytes.len() < size_bytes + (size as usize) {
            return Ok(None);
        }

        self.bytes.drain(..size_bytes);
        let encoded: Vec<u8> = self.bytes.drain(..size as usize).collect();

        let handler = self.inner.lock().unwrap().handler.clone();
        let decoded = handler.read(&encoded)?;

        let mut reader = std::io::Cursor::new(&decoded);
        Ok(Some(RawPacket {
            id: reader.decode::<i32>()?,
            data: reader.read_all()?,
        }))
    }

    /// Recieve & decode a packet if available.
    pub fn recieve_into<T>(&mut self) -> Result<Option<T>, ConnectionError>
    where
        T: TryFrom<RawPacket, Error = ConnectionError>,
    {
        self.recieve().map(|i| i.map(T::try_from).transpose())?
    }
}
