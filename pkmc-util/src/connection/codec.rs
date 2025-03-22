use std::io::{Read, Write};

use crate::{ReadExt, Transmutable as _};

pub trait PacketEncodable {
    fn packet_encode(self, writer: impl Write) -> std::io::Result<()>;
}

pub trait PacketDecodable
where
    Self: Sized,
{
    fn packet_decode(reader: impl Read) -> std::io::Result<Self>;
}

pub trait PacketEncoder {
    fn encode<V: PacketEncodable>(&mut self, value: V) -> std::io::Result<()>;
}

impl<W: Write> PacketEncoder for W {
    fn encode<V: PacketEncodable>(&mut self, value: V) -> std::io::Result<()> {
        value.packet_encode(self)
    }
}

pub trait PacketDecoder {
    fn decode<V: PacketDecodable>(&mut self) -> std::io::Result<V>;
}

impl<R: Read> PacketDecoder for R {
    fn decode<V: PacketDecodable>(&mut self) -> std::io::Result<V> {
        V::packet_decode(self)
    }
}

// Implement some basic codec types

impl PacketEncodable for bool {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        writer.write_all(&[if self { 1 } else { 0 }])
    }
}

impl PacketDecodable for bool {
    fn packet_decode(mut reader: impl Read) -> std::io::Result<Self> {
        match reader.read_const::<1>()? {
            [0] => Ok(false),
            [1] => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Boolean must be either 0 or 1",
            )),
        }
    }
}

impl<T: PacketEncodable> PacketEncodable for Option<T> {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        match self {
            None => writer.encode(false),
            Some(inner) => {
                writer.encode(true)?;
                writer.encode(inner)
            }
        }
    }
}

impl<T: PacketDecodable> PacketDecodable for Option<T> {
    fn packet_decode(mut reader: impl Read) -> std::io::Result<Self> {
        reader
            .decode::<bool>()?
            .then(|| reader.decode())
            .transpose()
    }
}

impl PacketEncodable for i32 {
    fn packet_encode(self, writer: impl Write) -> std::io::Result<()> {
        super::varint::write_varint(writer, self)
    }
}

impl PacketDecodable for i32 {
    fn packet_decode(reader: impl Read) -> std::io::Result<Self> {
        super::varint::read_varint(reader)
    }
}

impl PacketEncodable for i64 {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        let mut value: u64 = self.transmute();
        loop {
            let mut byte = value as u8 & 0x7F;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            writer.write_all(&[byte])?;
            if value == 0 {
                break;
            }
        }
        Ok(())
    }
}

impl PacketDecodable for i64 {
    fn packet_decode(_reader: impl Read) -> std::io::Result<Self> {
        unimplemented!()
    }
}

impl PacketEncodable for &str {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        writer.encode(self.len() as i32)?;
        writer.write_all(self.as_bytes())?;
        Ok(())
    }
}

impl PacketEncodable for &String {
    fn packet_encode(self, mut writer: impl Write) -> std::io::Result<()> {
        writer.encode::<&str>(self)
    }
}

impl PacketDecodable for String {
    fn packet_decode(mut reader: impl Read) -> std::io::Result<Self> {
        let len: i32 = reader.decode()?;
        let buf = reader.read_var(len as usize)?;
        String::from_utf8(buf.to_vec())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }
}
