use std::{
    collections::HashMap,
    io::{Read, Seek, Write},
};

use super::{tag::NBTTag, NBTError, NBTList, NBT};
use crate::{
    connection::{PacketDecodable, PacketEncodable},
    ReadExt as _, Transmutable,
};

impl NBT {
    fn read_tag(data: &mut impl Read, tag: NBTTag) -> Result<Self, NBTError> {
        match tag {
            NBTTag::End => Err(NBTError::UnexpectedEnd),
            NBTTag::Byte => Ok(NBT::Byte(i8::from_be_bytes(data.read_const()?))),
            NBTTag::Short => Ok(NBT::Short(i16::from_be_bytes(data.read_const()?))),
            NBTTag::Int => Ok(NBT::Int(i32::from_be_bytes(data.read_const()?))),
            NBTTag::Long => Ok(NBT::Long(i64::from_be_bytes(data.read_const()?))),
            NBTTag::Float => Ok(NBT::Float(f32::from_be_bytes(data.read_const()?))),
            NBTTag::Double => Ok(NBT::Double(f64::from_be_bytes(data.read_const()?))),
            NBTTag::ByteArray => Ok(NBT::ByteArray({
                let length = u32::from_be_bytes(data.read_const()?) as usize;
                data.read_var(length)?.transmute()
            })),
            NBTTag::String => Ok(NBT::String({
                let length = u16::from_be_bytes(data.read_const()?) as usize;
                String::from_utf8(data.read_var(length)?.to_vec())?
            })),
            NBTTag::List => Ok(NBT::List({
                let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
                let mut list = NBTList::new_with_tag(tag);
                (0..u32::from_be_bytes(data.read_const()?)).try_for_each(|_| {
                    let item = NBT::read_tag(data, tag)?;
                    list.push(item)?;
                    Ok::<(), NBTError>(())
                })?;
                list
            })),
            NBTTag::Compound => Ok(NBT::Compound({
                let mut compound = HashMap::new();
                loop {
                    let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
                    if tag == NBTTag::End {
                        break;
                    }
                    let length = u16::from_be_bytes(data.read_const()?) as usize;
                    let name = String::from_utf8(data.read_var(length)?.to_vec())?;
                    compound.insert(name, NBT::read_tag(data, tag)?);
                }
                compound
            })),
            NBTTag::IntArray => Ok(NBT::IntArray(
                (0..u32::from_be_bytes(data.read_const()?))
                    .map(|_| Ok(i32::from_be_bytes(data.read_const()?)))
                    .collect::<Result<Vec<_>, std::io::Error>>()?
                    .into_boxed_slice(),
            )),
            NBTTag::LongArray => Ok(NBT::LongArray(
                (0..u32::from_be_bytes(data.read_const()?))
                    .map(|_| Ok(i64::from_be_bytes(data.read_const()?)))
                    .collect::<Result<Vec<_>, std::io::Error>>()?
                    .into_boxed_slice(),
            )),
        }
    }

    pub fn read(mut data: impl Read, is_compressed: bool) -> Result<(String, NBT), NBTError> {
        if !is_compressed {
            let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
            let mut str_bytes = vec![0u8; u16::from_be_bytes(data.read_const()?) as usize];
            data.read_exact(&mut str_bytes)?;
            Ok((
                String::from_utf8(str_bytes)?,
                NBT::read_tag(&mut data, tag)?,
            ))
        } else {
            let mut data = flate2::read::GzDecoder::new(data);
            let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
            let mut str_bytes = vec![0u8; u16::from_be_bytes(data.read_const()?) as usize];
            data.read_exact(&mut str_bytes)?;
            Ok((
                String::from_utf8(str_bytes)?,
                NBT::read_tag(&mut data, tag)?,
            ))
        }
    }

    pub fn read_maybe_compressed(
        mut data: impl Read + Seek,
    ) -> Result<(bool, (String, NBT)), NBTError> {
        let ident: [u8; 2] = data.read_const()?;
        data.seek_relative(-2)?;
        let is_compressed = ident == [0x1F, 0x8B];
        Ok((is_compressed, NBT::read(data, is_compressed)?))
    }

    pub fn read_network(mut data: impl Read) -> Result<NBT, NBTError> {
        let tag = NBTTag::try_from(u8::from_be_bytes(data.read_const()?))?;
        NBT::read_tag(&mut data, tag)
    }

    fn write_tag(
        &self,
        name: Option<&str>,
        write_tag: bool,
        data: &mut impl Write,
    ) -> Result<(), NBTError> {
        if write_tag {
            data.write_all(&u8::from(self.tag()).to_be_bytes())?;
        }
        if let Some(name) = name {
            data.write_all(&(name.len() as u16).to_be_bytes())?;
            data.write_all(name.as_bytes())?;
        }
        match self {
            NBT::Byte(byte) => data.write_all(&byte.to_be_bytes())?,
            NBT::Short(short) => data.write_all(&short.to_be_bytes())?,
            NBT::Int(int) => data.write_all(&int.to_be_bytes())?,
            NBT::Long(long) => data.write_all(&long.to_be_bytes())?,
            NBT::Float(float) => data.write_all(&float.to_be_bytes())?,
            NBT::Double(double) => data.write_all(&double.to_be_bytes())?,
            NBT::String(string) => {
                data.write_all(&(string.len() as u16).to_be_bytes())?;
                data.write_all(string.as_bytes())?;
            }
            NBT::List(list) => {
                let Some(tag) = list.tag() else {
                    return Err(NBTError::InvalidList);
                };
                data.write_all(&u8::from(tag).to_be_bytes())?;
                data.write_all(&(list.len() as u32).to_be_bytes())?;
                for item in list.iter() {
                    item.write_tag(None, false, data)?;
                }
            }
            NBT::Compound(compound) => {
                compound
                    .iter()
                    .filter(|(_, value)| !matches!(value, NBT::List(list) if list.tag().is_none()))
                    .try_for_each(|(key, value)| value.write_tag(Some(key), true, data))?;
                data.write_all(&u8::from(NBTTag::End).to_be_bytes())?;
            }
            NBT::ByteArray(bytes) => {
                data.write_all(&(bytes.len() as u32).to_be_bytes())?;
                data.write_all(
                    &bytes
                        .iter()
                        .flat_map(|b| b.to_be_bytes())
                        .collect::<Vec<_>>(),
                )?;
            }
            NBT::IntArray(ints) => {
                data.write_all(&(ints.len() as u32).to_be_bytes())?;
                data.write_all(
                    &ints
                        .iter()
                        .flat_map(|i| i.to_be_bytes())
                        .collect::<Vec<_>>(),
                )?;
            }
            NBT::LongArray(longs) => {
                data.write_all(&(longs.len() as u32).to_be_bytes())?;
                data.write_all(
                    &longs
                        .iter()
                        .flat_map(|l| l.to_be_bytes())
                        .collect::<Vec<_>>(),
                )?;
            }
        }
        Ok(())
    }

    pub fn write(
        &self,
        name: &str,
        mut data: impl Write,
        is_compressed: bool,
    ) -> Result<(), NBTError> {
        if !is_compressed {
            self.write_tag(Some(name), true, &mut data)?;
            Ok(())
        } else {
            let mut encoder = flate2::write::GzEncoder::new(data, flate2::Compression::best());
            self.write_tag(Some(name), true, &mut encoder)?;
            encoder.finish()?;
            Ok(())
        }
    }

    pub fn write_network(&self, mut data: impl Write) -> Result<(), NBTError> {
        self.write_tag(None, true, &mut data)
    }
}

impl PacketEncodable for &NBT {
    fn packet_encode(self, writer: impl Write) -> std::io::Result<()> {
        self.write_network(writer)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

impl PacketDecodable for NBT {
    fn packet_decode(reader: impl Read) -> std::io::Result<Self> {
        NBT::read_network(reader).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}
