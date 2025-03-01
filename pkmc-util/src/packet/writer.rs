use std::io::Write;

use crate::{nbt::NBT, Position, Transmutable, UUID};

use super::BitSet;

pub fn write_varint(mut writer: impl Write, value: i32) -> std::io::Result<()> {
    let mut value: u32 = value.transmute();
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

pub const fn varint_size(mut value: i32) -> i32 {
    let mut bytes = 0;
    while value > 0 {
        value >>= 7;
        bytes += 1;
    }
    if bytes == 0 {
        1
    } else {
        bytes
    }
}

pub trait WriteExtPacket {
    fn write_varint(&mut self, value: i32) -> std::io::Result<()>;
    fn write_varlong(&mut self, value: i64) -> std::io::Result<()>;
    fn write_string(&mut self, string: &str) -> std::io::Result<()>;
    fn write_bool(&mut self, bool: bool) -> std::io::Result<()>;
    fn write_uuid(&mut self, uuid: &UUID) -> std::io::Result<()>;
    fn write_position(&mut self, position: &Position) -> std::io::Result<()>;
    fn write_bitset(&mut self, bitset: &BitSet) -> std::io::Result<()>;
    fn write_nbt(&mut self, nbt: &NBT) -> std::io::Result<()>;
}

impl<T: Write> WriteExtPacket for T {
    fn write_varint(&mut self, value: i32) -> std::io::Result<()> {
        write_varint(self, value)?;
        Ok(())
    }

    fn write_varlong(&mut self, value: i64) -> std::io::Result<()> {
        let mut value: u64 = value.transmute();
        loop {
            let mut byte = value as u8 & 0x7F;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.write_all(&[byte])?;
            if value == 0 {
                break;
            }
        }
        Ok(())
    }

    fn write_string(&mut self, string: &str) -> std::io::Result<()> {
        let buf = string.as_bytes();
        self.write_varint(
            buf.len()
                .try_into()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?,
        )?;
        self.write_all(buf)?;
        Ok(())
    }

    fn write_bool(&mut self, bool: bool) -> std::io::Result<()> {
        self.write_all(&[if bool { 1 } else { 0 }])?;
        Ok(())
    }

    fn write_uuid(&mut self, uuid: &UUID) -> std::io::Result<()> {
        self.write_all(&uuid.0)?;
        Ok(())
    }

    fn write_position(&mut self, position: &Position) -> std::io::Result<()> {
        let v: u64 = Transmutable::<u64>::transmute((position.x as i64) << 38)
            | (Transmutable::<u64>::transmute((position.y as i64) << 52) >> 52)
            | (Transmutable::<u64>::transmute((position.z as i64) << 38) >> 26);
        self.write_all(&v.to_be_bytes())?;
        Ok(())
    }

    fn write_bitset(&mut self, bitset: &BitSet) -> std::io::Result<()> {
        self.write_varint(
            bitset
                .inner()
                .len()
                .try_into()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?,
        )?;
        bitset
            .inner()
            .iter()
            .map(|l| self.write_all(&l.to_be_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    fn write_nbt(&mut self, nbt: &NBT) -> std::io::Result<()> {
        nbt.write_network(self)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::Position;

    use super::WriteExtPacket as _;

    fn writer_var_int(value: i32) -> std::io::Result<Vec<u8>> {
        let mut writer = Vec::new();
        writer.write_varint(value)?;
        Ok(writer)
    }

    #[test]
    #[rustfmt::skip]
    fn writer() -> std::io::Result<()> {
        assert_eq!(writer_var_int(0)?, &[0x00]);
        assert_eq!(writer_var_int(1)?, &[0x01]);
        assert_eq!(writer_var_int(2)?, &[0x02]);
        assert_eq!(writer_var_int(127)?, &[0x7f]);
        assert_eq!(writer_var_int(128)?, &[0x80, 0x01]);
        assert_eq!(writer_var_int(255)?, &[0xff, 0x01]);
        assert_eq!(writer_var_int(25565)?, &[0xdd, 0xc7, 0x01]);
        assert_eq!(writer_var_int(2097151)?, &[0xff, 0xff, 0x7f]);
        assert_eq!(writer_var_int(2147483647)?, &[0xff, 0xff, 0xff, 0xff, 0x07]);
        assert_eq!(writer_var_int(-1)?, &[0xff, 0xff, 0xff, 0xff, 0x0f]);
        assert_eq!(writer_var_int(-2147483648)?, &[0x80, 0x80, 0x80, 0x80, 0x08]);

        let mut pos_test = Vec::new();
        pos_test.write_position(&Position::new(18357644, 831, -20882616))?;
        assert_eq!(u64::from_be_bytes(pos_test.try_into().unwrap()), 0b0100011000000111011000110010110000010101101101001000001100111111);

        Ok(())
    }
}
