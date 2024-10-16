use anyhow::{anyhow, Result};
use std::io::Read;

use crate::uuid::UUID;

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

pub fn try_read_varint_ret_bytes(buf_start: &[u8]) -> Result<Option<(usize, i32)>> {
    match read_varint_ret_bytes(std::io::Cursor::new(buf_start)) {
        Ok(varint) => Ok(Some(varint)),
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(err) => Err(err.into()),
    }
}

pub struct PacketReader<D: Read> {
    data: D,
}

impl<D: Read> PacketReader<D> {
    pub fn new(data: D) -> Self {
        Self { data }
    }

    pub fn read_buf<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut data = [0u8; N];
        self.data.read_exact(&mut data)?;
        Ok(data)
    }

    pub fn read_vec(&mut self, length: usize) -> Result<Box<[u8]>> {
        let mut data = vec![0u8; length];
        self.data.read_exact(&mut data)?;
        Ok(data.into_boxed_slice())
    }

    pub fn read_vec_to_end(&mut self) -> Result<Box<[u8]>> {
        let mut data = Vec::new();
        self.data.read_to_end(&mut data)?;
        Ok(data.into_boxed_slice())
    }

    pub fn read_unsigned_byte(&mut self) -> Result<u8> {
        Ok(u8::from_be_bytes(self.read_buf()?))
    }

    pub fn read_signed_byte(&mut self) -> Result<i8> {
        Ok(i8::from_be_bytes(self.read_buf()?))
    }

    pub fn read_unsigned_short(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.read_buf()?))
    }

    pub fn read_long(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.read_buf()?))
    }

    pub fn read_var_int(&mut self) -> Result<i32> {
        Ok(read_varint(&mut self.data)?)
    }

    pub fn read_string(&mut self) -> Result<String> {
        let length = self.read_var_int()?;
        Ok(String::from_utf8(self.read_vec(length as usize)?.to_vec())?)
    }

    pub fn read_boolean(&mut self) -> Result<bool> {
        match self.read_unsigned_byte()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(anyhow!(
                "PacketReader::read_boolean invalid bool value {}",
                value
            )),
        }
    }

    pub fn read_uuid(&mut self) -> Result<UUID> {
        // FIXME: This is wrong, but it shouldn't really matter?
        // https://wiki.vg/Protocol#Type:UUID
        Ok(UUID(self.read_buf()?))
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::PacketReader;

    fn create_reader(data: &[u8]) -> PacketReader<std::io::Cursor<&[u8]>> {
        PacketReader::new(std::io::Cursor::new(data))
    }

    #[test]
    #[rustfmt::skip]
    fn reader() -> Result<()> {
        assert_eq!(create_reader(&[0x00]).read_var_int()?, 0);
        assert_eq!(create_reader(&[0x01]).read_var_int()?, 1);
        assert_eq!(create_reader(&[0x02]).read_var_int()?, 2);
        assert_eq!(create_reader(&[0x7f]).read_var_int()?, 127);
        assert_eq!(create_reader(&[0x80, 0x01]).read_var_int()?, 128);
        assert_eq!(create_reader(&[0xff, 0x01]).read_var_int()?, 255);
        assert_eq!(create_reader(&[0xdd, 0xc7, 0x01]).read_var_int()?, 25565);
        assert_eq!(create_reader(&[0xff, 0xff, 0x7f]).read_var_int()?, 2097151);
        assert_eq!(create_reader(&[0xff, 0xff, 0xff, 0xff, 0x07]).read_var_int()?, 2147483647);
        assert_eq!(create_reader(&[0xff, 0xff, 0xff, 0xff, 0x0f]).read_var_int()?, -1);
        assert_eq!(create_reader(&[0x80, 0x80, 0x80, 0x80, 0x08]).read_var_int()?, -2147483648);

        Ok(())
    }
}
