use anyhow::Result;
use std::{
    collections::VecDeque,
    io::{self, Read as _, Write as _},
    net::TcpStream,
};

use crate::{
    reader::{read_varint_ret_bytes, try_read_varint_ret_bytes},
    PacketReader, PacketWriter,
};

pub trait ServerboundPacket {
    const SERVERBOUND_ID: i32;

    fn serverbound_id(&self) -> i32 {
        Self::SERVERBOUND_ID
    }

    fn packet_read(reader: &mut PacketReader<std::io::Cursor<&[u8]>>) -> Result<Self>
    where
        Self: Sized;
}

pub trait ClientboundPacket {
    const CLIENTBOUND_ID: i32;

    fn clientbound_id(&self) -> i32 {
        Self::CLIENTBOUND_ID
    }

    fn packet_write(&self, writer: &mut PacketWriter<Vec<u8>>) -> Result<()>;
}

#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
    closed: bool,
    bytes: VecDeque<u8>,
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

    pub fn reader(&self) -> PacketReader<std::io::Cursor<&[u8]>> {
        PacketReader::new(std::io::Cursor::new(&self.data))
    }
}

impl Connection {
    pub fn new(stream: TcpStream) -> Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            closed: false,
            bytes: VecDeque::new(),
        })
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    fn recieve_bytes(&mut self) -> Result<()> {
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
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    pub fn send(&mut self, packet: impl ClientboundPacket) -> Result<()> {
        // TODO: Rewrite please, I'm sorry for this, this is pretty dumb.
        let mut writer_data = PacketWriter::new_empty();
        packet.packet_write(&mut writer_data)?;

        let mut writer_id = PacketWriter::new_empty();
        writer_id.write_var_int(packet.clientbound_id())?;

        let contents = writer_id
            .into_inner()
            .into_iter()
            .chain(writer_data.into_inner())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        PacketWriter::new(&mut self.stream).write_var_int(contents.len() as i32)?;
        self.stream.write_all(&contents)?;
        self.stream.flush()?;

        Ok(())
    }

    pub fn recieve(&mut self) -> Result<Option<RawPacket>> {
        self.recieve_bytes()?;

        // TODO: Rewrite please, I'm sorry for this as well, This is way more dumb.
        let front = [
            self.bytes.front(),
            self.bytes.get(1),
            self.bytes.get(2),
            self.bytes.get(3),
            self.bytes.get(4),
        ]
        .into_iter()
        .filter_map(|v| v.cloned())
        .collect::<Vec<_>>();
        let Some((length_bytes, length)) = try_read_varint_ret_bytes(&front)? else {
            return Ok(None);
        };

        if self.bytes.len() < length_bytes + length as usize {
            return Ok(None);
        }

        (0..length_bytes).for_each(|_| {
            self.bytes.pop_front();
        });

        let mut data = vec![0u8; length as usize];
        self.bytes.read_exact(&mut data)?;

        let (id_length, id) = read_varint_ret_bytes(std::io::Cursor::new(&data))?;
        // ;-;
        (0..id_length).for_each(|_| {
            data.remove(0);
        });

        Ok(Some(RawPacket {
            id,
            data: data.into_boxed_slice(),
        }))
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
            type Error = anyhow::Error;

            fn try_from(value: $crate::connection::RawPacket) -> std::result::Result<Self, Self::Error> {
                use $crate::connection::ServerboundPacket as _;
                let mut reader = value.reader();
                match value.id {
                    $(
                        <$type>::SERVERBOUND_ID => Ok(Self::$name(<$type>::packet_read(&mut reader)?)),
                    )*
                    _ => Err(anyhow::anyhow!("{} unsupported packet {}", stringify!($enum_name), value.id)),
                }
            }
        }
    }
}
