use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

use crate::{packet::try_read_varint_ret_bytes, ReadExt};

use super::{
    handler::{PacketHandler, UncompressedPacketHandler},
    ClientboundPacket, ConnectionError, RawPacket, ReadExtPacket, WriteExtPacket,
};

#[derive(Debug)]
struct ConnectionInner {
    stream: Option<TcpStream>,
    handler: PacketHandler,
}

#[derive(Debug, Clone)]
pub struct ConnectionSender {
    inner: Arc<Mutex<ConnectionInner>>,
}

impl ConnectionSender {
    pub fn is_closed(&self) -> bool {
        self.inner.lock().unwrap().stream.is_none()
    }

    pub fn send(&self, packet: &impl ClientboundPacket) -> Result<(), ConnectionError> {
        let raw: RawPacket = packet.raw_packet()?;
        let bytes = raw.into_bytes();

        let handler = self.inner.lock().unwrap().handler.clone();

        let encoded = handler.write(&bytes)?;

        let mut with_size = Vec::new();
        with_size.write_varint(encoded.len() as i32)?;
        with_size.write_all(&encoded)?;

        let mut inner = self.inner.lock().unwrap();
        let Some(stream) = inner.stream.as_mut() else {
            return Ok(());
        };
        match stream.write_all(&with_size) {
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

#[derive(Debug)]
pub struct Connection {
    inner: Arc<Mutex<ConnectionInner>>,
    bytes: VecDeque<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Result<Self, ConnectionError> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(ConnectionInner {
                stream: Some(stream),
                handler: PacketHandler::Uncompressed(UncompressedPacketHandler),
            })),
            bytes: VecDeque::new(),
        })
    }

    pub fn sender(&self) -> ConnectionSender {
        ConnectionSender {
            inner: self.inner.clone(),
        }
    }

    pub fn set_packet_handler(&self, handler: PacketHandler) {
        self.inner.lock().unwrap().handler = handler;
    }

    pub fn is_closed(&self) -> bool {
        self.inner.lock().unwrap().stream.is_none()
    }

    pub fn close(&self) {
        self.inner.lock().unwrap().stream = None;
    }

    pub fn send(&self, packet: &impl ClientboundPacket) -> Result<(), ConnectionError> {
        self.sender().send(packet)
    }

    fn recieve_bytes(&mut self) -> Result<(), ConnectionError> {
        // TODO: What is best size for this?
        let mut buf = [0u8; 1024];
        let mut inner = self.inner.lock().unwrap();
        let Some(stream) = inner.stream.as_mut() else {
            return Ok(());
        };
        loop {
            match stream.read(&mut buf) {
                Ok(0) => {
                    inner.stream = None;
                    break;
                }
                Ok(n) => self.bytes.extend(&buf[..n]),
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
            id: reader.read_varint()?,
            data: reader.read_all()?,
        }))
    }

    pub fn recieve_into<T>(&mut self) -> Result<Option<T>, ConnectionError>
    where
        T: TryFrom<RawPacket, Error = ConnectionError>,
    {
        self.recieve().map(|i| i.map(T::try_from).transpose())?
    }
}
