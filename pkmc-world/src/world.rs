#![allow(unused)]

use std::{collections::HashMap, fs::File, io::Seek, marker::PhantomData, path::PathBuf};

use pkmc_nbt::{NBTError, NBT};
use pkmc_util::{PackedArray, ReadExt};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorldError {
    #[error("{0:?}")]
    IoError(#[from] std::io::Error),
    #[error("Region chunk unknown compression \"{0}\"")]
    RegionUnknownCompression(u8),
    #[error("Region chunk unsupported compression \"{0}\"")]
    RegionUnsupportedCompression(String),
    #[error("{0:?}")]
    NBTError(#[from] NBTError),
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChunkSectionBlockStatesPaletteEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Properties")]
    properties: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct ChunkSectionBlockStates {
    palette: Box<[ChunkSectionBlockStatesPaletteEntry]>,
    data: Option<Box<[i64]>>,
}

impl ChunkSectionBlockStates {
    fn get_block(&self, x: u8, y: u8, z: u8) -> ChunkSectionBlockStatesPaletteEntry {
        match self.palette.len() {
            0 => panic!(),
            1 => self.palette[0].clone(),
            palette_count => {
                let mut packed_indices = PackedArray::from_inner(
                    // TODO: Is this safe? lmao
                    unsafe {
                        std::mem::transmute::<Box<[i64]>, Box<[u64]>>(
                            self.data.as_ref().unwrap().clone(),
                        )
                    },
                    PackedArray::bits_per_entry(palette_count as u64 - 1),
                    16 * 16 * 16,
                );
                let index_index = ((y & 15) as usize) * 16 * 16 + (z as usize) * 16 + (x as usize);
                let index = packed_indices.get_unchecked(index_index) as usize;
                self.palette[index].clone()
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChunkSection {
    #[serde(rename = "Y")]
    y: i32,
    block_states: ChunkSectionBlockStates,
}

#[derive(Debug, Deserialize)]
pub struct Chunk {
    #[serde(rename = "DataVersion")]
    data_version: i32,
    #[serde(rename = "xPos")]
    x_pos: i32,
    #[serde(rename = "zPos")]
    z_pos: i32,
    #[serde(rename = "yPos")]
    y_pos: i32,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "LastUpdate")]
    last_update: i64,
    sections: Box<[ChunkSection]>,
}

impl Chunk {
    pub fn get_block(&self, x: u8, y: i16, z: u8) -> ChunkSectionBlockStatesPaletteEntry {
        let section_y = (y / 16) as i32;
        let Some(section) = self.sections.iter().find(|section| section.y == section_y) else {
            return ChunkSectionBlockStatesPaletteEntry {
                name: "minecraft:air".to_owned(),
                properties: None,
            };
        };
        section.block_states.get_block(x, (y & 15) as u8, z)
    }
}

#[derive(Debug)]
struct RegionLoader<T> {
    file: File,
    region_x: i32,
    region_z: i32,
    locations: [(u32, u32); 1024],
    _phantom: PhantomData<T>,
}

impl<T> RegionLoader<T> {
    pub fn load(mut file: File, region_x: i32, region_z: i32) -> Result<Self, WorldError> {
        let mut locations = [(0, 0); 1024];
        file.rewind()?;
        locations.iter_mut().try_for_each(|(offset, length)| {
            let data = u32::from_be_bytes(file.read_const()?);
            *offset = ((data & 0xFFFFFF00) >> 8) * 0x1000;
            *length = (data & 0x000000FF) * 0x1000;
            Ok::<_, WorldError>(())
        })?;
        Ok(Self {
            file,
            region_x,
            region_z,
            locations,
            _phantom: PhantomData::default(),
        })
    }

    pub fn read(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<Box<[u8]>>, WorldError> {
        let (offset, length) = self.locations[(chunk_x as usize) + (chunk_z as usize) * 32];
        if offset == 0 || length == 0 {
            return Ok(None);
        }
        self.file.seek(std::io::SeekFrom::Start(offset as u64))?;
        let length = u32::from_be_bytes(self.file.read_const()?);
        if length <= 1 {
            return Ok(None);
        }
        let compression_type = u8::from_be_bytes(self.file.read_const()?);
        let compressed_data = self.file.read_var((length as usize) - 1)?;
        match compression_type {
            1 => Err(WorldError::RegionUnsupportedCompression("GZip".to_owned())),
            2 => Ok(Some(
                flate2::read::ZlibDecoder::new(std::io::Cursor::new(compressed_data)).read_all()?,
            )),
            3 => Ok(Some(compressed_data)),
            4 => Err(WorldError::RegionUnsupportedCompression("LZ4".to_owned())),
            127 => {
                let mut data = std::io::Cursor::new(&compressed_data);
                let string_length = u16::from_be_bytes(data.read_const()?);
                let string_buf = data.read_var(string_length as usize)?;
                Err(WorldError::RegionUnsupportedCompression(format!(
                    "Custom {}",
                    String::from_utf8_lossy(&string_buf)
                )))
            }
            _ => Err(WorldError::RegionUnknownCompression(compression_type)),
        }
    }

    fn read_nbt(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<(String, NBT)>, WorldError> {
        Ok(self
            .read(chunk_x, chunk_z)?
            .map(|data| NBT::read(std::io::Cursor::new(data), false))
            .transpose()?)
    }
}

impl RegionLoader<Chunk> {
    fn read_chunk(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<Chunk>, WorldError> {
        let Some(chunk): Option<Chunk> = self
            .read_nbt(chunk_x, chunk_z)?
            .map(|nbt| pkmc_nbt::from_nbt(nbt.1))
            .transpose()?
        else {
            return Ok(None);
        };
        if chunk.status != "minecraft:full" {
            return Ok(None);
        }
        Ok(Some(chunk))
    }
}

#[derive(Debug)]
pub struct Level {
    identifier: String,
    root: PathBuf,
}

impl Level {
    fn get_region_loader_chunk(
        &self,
        region_x: i32,
        region_z: i32,
    ) -> Result<Option<RegionLoader<Chunk>>, WorldError> {
        let mut path = self.root.clone();
        path.push("region");
        path.push(format!("r.{}.{}.mca", region_x, region_z));
        let file = match std::fs::File::open(path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            result => result,
        }?;
        Ok(Some(RegionLoader::load(file, region_x, region_z)?))
    }
}

#[derive(Debug)]
pub struct World {
    root: PathBuf,
    levels: Vec<Level>,
}

impl World {
    pub fn load<P: Into<PathBuf>>(root: P) -> Result<Self, WorldError> {
        let root = root.into();
        Ok(Self {
            root: root.clone(),
            levels: vec![Level {
                identifier: "minecraft:overworld".to_owned(),
                root: root.clone(),
            }],
        })
    }

    fn get_level(&self, identifier: &str) -> Option<&Level> {
        self.levels
            .iter()
            .find(|level| level.identifier == identifier)
    }
}

#[cfg(test)]
mod test {
    use serde::Deserialize;

    use crate::world::World;

    use super::WorldError;

    #[test]
    fn test_worldload() -> Result<(), WorldError> {
        // Sorry I am not putting the whole world in this repo.
        // It is just a normally created minecraft 1.21.1 world.
        const WORLD_PATH: &str = "/home/vulae/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/Fabulously Optimized/.minecraft/saves/pkmc world/";

        let world = World::load(WORLD_PATH)?;
        println!("{:#?}", world);

        let level = world.get_level("minecraft:overworld").unwrap();
        let mut region_loader = level.get_region_loader_chunk(0, 0)?.unwrap();
        let chunk_data = region_loader.read_chunk(0, 0)?.unwrap();
        println!("{:?}", chunk_data.get_block(0, 0, 0));
        println!("{:?}", chunk_data.get_block(0, 1, 0));
        println!("{:?}", chunk_data.get_block(9, 2, 14));

        Ok(())
    }
}
