use std::io::Read;

use crate::{Position, ReadExt as _, UUID};

pub fn read_varint_ret_bytes(mut reader: impl Read) -> std::io::Result<(usize, i32)> {
    let mut bytes = 0;
    let mut value = 0;
    let mut position = 0;
    loop {
        let mut byte_buf = [0u8; 1];
        reader.read_exact(&mut byte_buf)?;
        let byte = u8::from_be_bytes(byte_buf);
        bytes += 1;
        value |= ((byte & 0x7F) as i32) << position;
        if byte & 0x80 == 0 {
            break;
        }
        position += 7;
        if position >= 32 {
            panic!("VarInt is too big");
        }
    }
    Ok((bytes, value))
}

pub fn read_varint(reader: impl Read) -> std::io::Result<i32> {
    Ok(read_varint_ret_bytes(reader)?.1)
}

pub fn try_read_varint_ret_bytes(data: &[u8]) -> std::io::Result<Option<(usize, i32)>> {
    match read_varint_ret_bytes(data) {
        Ok(varint) => Ok(Some(varint)),
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(err) => Err(err),
    }
}

pub trait ReadExtPacket {
    fn read_varint(&mut self) -> std::io::Result<i32>;
    fn read_string(&mut self) -> std::io::Result<String>;
    fn read_bool(&mut self) -> std::io::Result<bool>;
    fn read_uuid(&mut self) -> std::io::Result<UUID>;
    fn read_position(&mut self) -> std::io::Result<Position>;
}

impl<T: Read> ReadExtPacket for T {
    fn read_varint(&mut self) -> std::io::Result<i32> {
        read_varint(self)
    }

    fn read_string(&mut self) -> std::io::Result<String> {
        let length = self.read_varint()?;
        let buf = self.read_var(
            length
                .try_into()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?,
        )?;
        let str = String::from_utf8(buf.to_vec())
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        Ok(str)
    }

    fn read_bool(&mut self) -> std::io::Result<bool> {
        match u8::from_le_bytes(self.read_const()?) {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid boolean",
            )),
        }
    }

    fn read_uuid(&mut self) -> std::io::Result<UUID> {
        Ok(UUID(self.read_const()?))
    }

    fn read_position(&mut self) -> std::io::Result<Position> {
        let v = i64::from_be_bytes(self.read_const()?);
        Ok(Position {
            x: (v >> 38) as i32,
            y: (v << 52 >> 52) as i16,
            z: (v << 26 >> 38) as i32,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::packet::ReadExtPacket as _;

    fn create_reader(data: &[u8]) -> std::io::Cursor<&[u8]> {
        std::io::Cursor::new(data)
    }

    #[test]
    #[rustfmt::skip]
    fn reader() -> std::io::Result<()> {
        assert_eq!(create_reader(&[0x00]).read_varint()?, 0);
        assert_eq!(create_reader(&[0x01]).read_varint()?, 1);
        assert_eq!(create_reader(&[0x02]).read_varint()?, 2);
        assert_eq!(create_reader(&[0x7f]).read_varint()?, 127);
        assert_eq!(create_reader(&[0x80, 0x01]).read_varint()?, 128);
        assert_eq!(create_reader(&[0xff, 0x01]).read_varint()?, 255);
        assert_eq!(create_reader(&[0xdd, 0xc7, 0x01]).read_varint()?, 25565);
        assert_eq!(create_reader(&[0xff, 0xff, 0x7f]).read_varint()?, 2097151);
        assert_eq!(create_reader(&[0xff, 0xff, 0xff, 0xff, 0x07]).read_varint()?, 2147483647);
        assert_eq!(create_reader(&[0xff, 0xff, 0xff, 0xff, 0x0f]).read_varint()?, -1);
        assert_eq!(create_reader(&[0x80, 0x80, 0x80, 0x80, 0x08]).read_varint()?, -2147483648);

        Ok(())
    }
}
