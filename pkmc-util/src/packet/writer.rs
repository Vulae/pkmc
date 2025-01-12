use std::io::Write;

use crate::{nbt::NBT, Transmutable, UUID};

use super::{BitSet, Position};

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
        let x = Transmutable::<u32>::transmute(position.x) as u64;
        let y = Transmutable::<u16>::transmute(position.y) as u64;
        let z = Transmutable::<u32>::transmute(position.z) as u64;
        let v: u64 = ((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF);
        self.write_all(&v.to_be_bytes())?;
        Ok(())
    }

    fn write_bitset(&mut self, bitset: &BitSet) -> std::io::Result<()> {
        self.write_varint(
            bitset
                .num_longs()
                .try_into()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?,
        )?;
        bitset
            .longs_iter()
            .map(|l| self.write_all(&l.to_be_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    fn write_nbt(&mut self, nbt: &NBT) -> std::io::Result<()> {
        self.write_all(
            &nbt.to_bytes_network()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::WriteExtPacket as _;

    fn writer_var_int(value: i32) -> std::io::Result<Vec<u8>> {
        let mut writer = Vec::new();
        writer.write_varint(value)?;
        Ok(writer)
    }

    #[test]
    #[rustfmt::skip]
    fn reader() -> std::io::Result<()> {
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

        Ok(())
    }
}
