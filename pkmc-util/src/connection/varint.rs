use std::io::{Read, Write};

use crate::Transmutable as _;

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

#[cfg(test)]
mod test {
    use super::{read_varint, write_varint};

    #[test]
    #[rustfmt::skip]
    fn reader() -> std::io::Result<()> {
        assert_eq!(read_varint(std::io::Cursor::new(&[0x00]))?, 0);
        assert_eq!(read_varint(std::io::Cursor::new(&[0x01]))?, 1);
        assert_eq!(read_varint(std::io::Cursor::new(&[0x02]))?, 2);
        assert_eq!(read_varint(std::io::Cursor::new(&[0x7f]))?, 127);
        assert_eq!(read_varint(std::io::Cursor::new(&[0x80, 0x01]))?, 128);
        assert_eq!(read_varint(std::io::Cursor::new(&[0xff, 0x01]))?, 255);
        assert_eq!(read_varint(std::io::Cursor::new(&[0xdd, 0xc7, 0x01]))?, 25565);
        assert_eq!(read_varint(std::io::Cursor::new(&[0xff, 0xff, 0x7f]))?, 2097151);
        assert_eq!(read_varint(std::io::Cursor::new(&[0xff, 0xff, 0xff, 0xff, 0x07]))?, 2147483647);
        assert_eq!(read_varint(std::io::Cursor::new(&[0xff, 0xff, 0xff, 0xff, 0x0f]))?, -1);
        assert_eq!(read_varint(std::io::Cursor::new(&[0x80, 0x80, 0x80, 0x80, 0x08]))?, -2147483648);

        Ok(())
    }

    fn writer_var_int(value: i32) -> std::io::Result<Vec<u8>> {
        let mut writer = Vec::new();
        write_varint(&mut writer, value)?;
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

        Ok(())
    }
}
