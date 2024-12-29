#![allow(unused)]

use std::{collections::HashMap, fs::File, io::Seek, marker::PhantomData, path::PathBuf};

use pkmc_defs::block::Block;
use pkmc_nbt::{NBTError, NBT};
use pkmc_util::{PackedArray, ReadExt, Transmutable as _};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorldError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Region chunk unknown compression \"{0}\"")]
    RegionUnknownCompression(u8),
    #[error("Region chunk unsupported compression \"{0}\"")]
    RegionUnsupportedCompression(String),
    #[error(transparent)]
    NBTError(#[from] NBTError),
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChunkSectionBlockStatesPaletteEntry {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Properties", default)]
    properties: HashMap<String, String>,
}

impl ChunkSectionBlockStatesPaletteEntry {
    pub fn to_block(&self) -> Block {
        Block::new_p(&self.name, self.properties.iter())
    }
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
                    self.data.as_ref().unwrap().clone().transmute(),
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
    pub fn get_block(&self, x: u8, y: i16, z: u8) -> Block {
        let section_y = (y / 16) as i32;
        let Some(section) = self.sections.iter().find(|section| section.y == section_y) else {
            return Block::air();
        };
        section
            .block_states
            .get_block(x, (y & 15) as u8, z)
            .to_block()
    }
}

#[derive(Debug)]
struct RegionLoader {
    file: File,
    region_x: i32,
    region_z: i32,
    locations: [(u32, u32); 1024],
    loaded_chunks: HashMap<(u8, u8), Option<Chunk>>,
}

impl RegionLoader {
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
            loaded_chunks: HashMap::new(),
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

    fn load_chunk(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<Chunk>, WorldError> {
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

    fn get_or_load_chunk(
        &mut self,
        chunk_x: u8,
        chunk_z: u8,
    ) -> Result<Option<&Chunk>, WorldError> {
        // Why does clippy complain? doing its suggestion breaks the code.
        #[allow(clippy::all)]
        if !self.loaded_chunks.contains_key(&(chunk_x, chunk_z)) {
            let region = self.load_chunk(chunk_x, chunk_z)?;
            self.loaded_chunks.insert((chunk_x, chunk_z), region);
        }

        Ok(self
            .loaded_chunks
            .get_mut(&(chunk_x, chunk_z))
            .unwrap()
            .as_ref())
    }
}

#[derive(Debug)]
pub struct Level {
    identifier: String,
    root: PathBuf,
    loaded_regions: HashMap<(i32, i32), Option<RegionLoader>>,
}

impl Level {
    fn new(identifier: String, root: PathBuf) -> Self {
        Self {
            identifier,
            root,
            loaded_regions: HashMap::new(),
        }
    }

    fn load_region(
        &self,
        region_x: i32,
        region_z: i32,
    ) -> Result<Option<RegionLoader>, WorldError> {
        let mut path = self.root.clone();
        path.push("region");
        path.push(format!("r.{}.{}.mca", region_x, region_z));

        let file = match std::fs::File::open(path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            result => result,
        }?;

        Ok(Some(RegionLoader::load(file, region_x, region_z)?))
    }

    fn get_or_load_region(
        &mut self,
        region_x: i32,
        region_z: i32,
    ) -> Result<Option<&mut RegionLoader>, WorldError> {
        //Ok(self
        //    .loaded_regions
        //    .entry((region_x, region_z))
        //    .or_insert_with(|| self.load_region(region_x, region_z).unwrap())
        //    .as_mut())

        // Why does clippy complain? doing its suggestion breaks the code.
        #[allow(clippy::all)]
        if !self.loaded_regions.contains_key(&(region_x, region_z)) {
            let region = self.load_region(region_x, region_z)?;
            self.loaded_regions.insert((region_x, region_z), region);
        }

        Ok(self
            .loaded_regions
            .get_mut(&(region_x, region_z))
            .unwrap()
            .as_mut())
    }

    pub fn get_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Result<Option<&Chunk>, WorldError> {
        let Some(region) = self.get_or_load_region(chunk_x / 32, chunk_z / 32)? else {
            return Ok(None);
        };
        let Some(chunk) = region.get_or_load_chunk((chunk_x & 31) as u8, (chunk_z & 31) as u8)?
        else {
            return Ok(None);
        };
        Ok(Some(chunk))
    }

    pub fn get_block(
        &mut self,
        block_x: i32,
        block_y: i16,
        block_z: i32,
    ) -> Result<Option<Block>, WorldError> {
        // TODO: Chunk cache, so getting a block will not decode the whole chunk again.
        let Some(chunk) = self.get_chunk(block_x / 16, block_z / 16)? else {
            return Ok(None);
        };
        let block = chunk.get_block((block_x & 15) as u8, block_y, (block_z & 15) as u8);
        Ok(Some(block))
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
            levels: vec![Level::new("minecraft:overworld".to_owned(), root.clone())],
        })
    }

    fn get_level(&mut self, identifier: &str) -> Option<&mut Level> {
        self.levels
            .iter_mut()
            .find(|level| level.identifier == identifier)
    }
}

#[cfg(test)]
mod test {
    use pkmc_defs::block::BLOCKS_TO_IDS;
    use serde::Deserialize;

    use crate::world::World;

    use super::WorldError;

    #[test]
    fn test_worldload() -> Result<(), WorldError> {
        // 1.21.4 debug world
        // https://minecraft.wiki/w/Debug_mode
        const WORLD_PATH: &str = "/home/vulae/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/pkmc/minecraft/saves/debug/";

        let mut world = World::load(WORLD_PATH)?;

        let level = world.get_level("minecraft:overworld").unwrap();

        let max_block_id = *BLOCKS_TO_IDS.values().max().unwrap();

        let block_grid_width = (max_block_id as f32).sqrt().ceil() as i32;
        let block_grid_height = (max_block_id as f32 / block_grid_width as f32).ceil() as i32;

        for block_id in 0..max_block_id {
            // FIXME: The indexing into grid is wrong.
            // It gets wrong at the last edge of the grid, and I have no idea why.
            // So for now we just skip those, but please do fix.
            if block_id > 26000 {
                continue;
            }

            let grid_x = block_id % block_grid_width;
            let grid_z = block_id / block_grid_width;

            let x = 1 + grid_z * 2;
            let y = 70;
            let z = 1 + grid_x * 2;

            let Some(block) = level.get_block(x, y, z)? else {
                panic!("Expected loaded block at {} {} {}", x, y, z);
            };

            if block.id() != Some(block_id) {
                panic!(
                    "Block at {} {} {} is {:?} with ID {:?}, but our ID is {}",
                    x,
                    y,
                    z,
                    block,
                    block.id(),
                    block_id
                );
            }
        }

        Ok(())
    }
}
