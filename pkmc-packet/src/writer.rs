use anyhow::Result;
use pkmc_nbt::NBT;
use pkmc_util::UUID;
use std::io::Write;

use super::{BitSet, Position};

pub fn write_varint(mut writer: impl Write, value: i32) -> Result<()> {
    let mut value = unsafe { std::mem::transmute::<i32, u32>(value) };
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

pub struct PacketWriter<D: Write> {
    data: D,
}

impl PacketWriter<Vec<u8>> {
    pub fn new_empty() -> Self {
        Self { data: Vec::new() }
    }
}

impl<D: Write> PacketWriter<D> {
    pub fn new(data: D) -> Self {
        Self { data }
    }

    pub fn into_inner(self) -> D {
        self.data
    }

    pub fn write_buf(&mut self, buf: &[u8]) -> Result<()> {
        self.data.write_all(buf)?;
        Ok(())
    }

    pub fn write_signed_byte(&mut self, value: i8) -> Result<()> {
        self.write_buf(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_unsigned_byte(&mut self, value: u8) -> Result<()> {
        self.write_buf(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_short(&mut self, value: i16) -> Result<()> {
        self.write_buf(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_int(&mut self, value: i32) -> Result<()> {
        self.write_buf(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_long(&mut self, value: i64) -> Result<()> {
        self.write_buf(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_float(&mut self, value: f32) -> Result<()> {
        self.write_buf(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_double(&mut self, value: f64) -> Result<()> {
        self.write_buf(&value.to_be_bytes())?;
        Ok(())
    }

    pub fn write_var_int(&mut self, value: i32) -> Result<()> {
        write_varint(&mut self.data, value)
    }

    pub fn write_string(&mut self, str: &str) -> Result<()> {
        self.write_var_int(str.len() as i32)?;
        self.write_buf(str.as_bytes())?;
        Ok(())
    }

    pub fn write_uuid(&mut self, uuid: &UUID) -> Result<()> {
        self.write_buf(&uuid.0)?;
        Ok(())
    }

    pub fn write_boolean(&mut self, bool: bool) -> Result<()> {
        self.write_unsigned_byte(if bool { 0x01 } else { 0x00 })?;
        Ok(())
    }

    pub fn write_position(&mut self, position: Position) -> Result<()> {
        let x = unsafe { std::mem::transmute::<i32, u32>(position.x) } as u64;
        let y = unsafe { std::mem::transmute::<i16, u16>(position.y) } as u64;
        let z = unsafe { std::mem::transmute::<i32, u32>(position.z) } as u64;
        let v: u64 = ((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF);
        self.write_buf(&v.to_be_bytes())?;
        Ok(())
    }

    pub fn write_bitset(&mut self, bitset: &BitSet) -> Result<()> {
        self.write_var_int(bitset.num_longs() as i32)?;
        bitset
            .longs_iter()
            .map(|l| self.write_buf(&l.to_be_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    pub fn write_nbt(&mut self, nbt: &NBT) -> Result<()> {
        self.write_buf(&nbt.to_bytes_network()?)?;
        Ok(())
    }
}

pub fn write_packet(id: u8, data: &[u8]) -> Result<Box<[u8]>> {
    let mut writer = PacketWriter::new_empty();
    writer.write_var_int(data.len() as i32 + 1)?;
    writer.write_unsigned_byte(id)?;
    writer.write_buf(data)?;
    Ok(writer.into_inner().into_boxed_slice())
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::PacketWriter;

    fn create_writer() -> PacketWriter<Vec<u8>> {
        PacketWriter::new(Vec::new())
    }

    macro_rules! writer_var_int {
        ($value:expr) => {{
            let mut writer = create_writer();
            writer.write_var_int($value)?;
            writer.into_inner()
        }};
    }

    #[test]
    #[rustfmt::skip]
    fn reader() -> Result<()> {
        assert_eq!(writer_var_int!(0), &[0x00]);
        assert_eq!(writer_var_int!(1), &[0x01]);
        assert_eq!(writer_var_int!(2), &[0x02]);
        assert_eq!(writer_var_int!(127), &[0x7f]);
        assert_eq!(writer_var_int!(128), &[0x80, 0x01]);
        assert_eq!(writer_var_int!(255), &[0xff, 0x01]);
        assert_eq!(writer_var_int!(25565), &[0xdd, 0xc7, 0x01]);
        assert_eq!(writer_var_int!(2097151), &[0xff, 0xff, 0x7f]);
        assert_eq!(writer_var_int!(2147483647), &[0xff, 0xff, 0xff, 0xff, 0x07]);
        assert_eq!(writer_var_int!(-1), &[0xff, 0xff, 0xff, 0xff, 0x0f]);
        assert_eq!(writer_var_int!(-2147483648), &[0x80, 0x80, 0x80, 0x80, 0x08]);

        Ok(())
    }
}
