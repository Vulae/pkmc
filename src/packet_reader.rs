use anyhow::Result;
use std::io::Read;

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

    pub fn read_unsigned_byte(&mut self) -> Result<u8> {
        Ok(u8::from_be_bytes(self.read_buf()?))
    }

    pub fn read_unsigned_short(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.read_buf()?))
    }

    pub fn read_long(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(self.read_buf()?))
    }

    pub fn read_var_int(&mut self) -> Result<i32> {
        let mut value = 0;
        let mut position = 0;
        loop {
            let byte = self.read_unsigned_byte()?;
            value |= ((byte & 0x7F) as i32) << position;
            if byte & 0x80 == 0 {
                break;
            }
            position += 7;
            if position >= 32 {
                panic!("VarInt is too big");
            }
        }
        Ok(value)
    }

    pub fn read_string(&mut self) -> Result<String> {
        let length = self.read_var_int()?;
        Ok(String::from_utf8(self.read_vec(length as usize)?.to_vec())?)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use crate::packet_reader::PacketReader;

    fn create_reader(data: &[u8]) -> PacketReader<std::io::Cursor<&[u8]>> {
        PacketReader::new(std::io::Cursor::new(data))
    }

    #[test]
    #[rustfmt::skip]
    fn reader() -> Result<()> {
        assert_eq!(create_reader(&[0x00]).read_var_int()?, 0);
        assert_eq!(create_reader(&[0x01]).read_var_int()?, 1);
        assert_eq!(create_reader(&[0x02]).read_var_int()?, 2);
        assert_eq!(create_reader(&[0x7F]).read_var_int()?, 127);
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
