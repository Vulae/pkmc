use std::{collections::HashMap, fs::File, io::Seek, path::PathBuf};

use pkmc_defs::{block::Block, generated::PALETTED_DATA_BLOCKS_INDIRECT};
use pkmc_util::{
    nbt::{from_nbt, NBTError, NBT},
    PackedArray, ReadExt, Transmutable,
};
use serde::Deserialize;
use thiserror::Error;

pub const REGION_SIZE: usize = 32;
pub const CHUNKS_PER_REGION: usize = REGION_SIZE * REGION_SIZE;
pub const CHUNK_SIZE: usize = 16;
pub const SECTION_SIZE: usize = 16;
pub const BLOCKS_PER_SECTION: usize = SECTION_SIZE * SECTION_SIZE * SECTION_SIZE;
pub const BIOMES_PER_SECTION: usize = 64;

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
struct ChunkSectionBlockStates {
    palette: Box<[Block]>,
    data: Option<Box<[i64]>>,
}

impl ChunkSectionBlockStates {
    fn get_block_palette_index_by_index(&self, index: usize) -> usize {
        // FIXME: get_block_palette_index_by_index sometimes returns out of bounds index :(
        match self.palette.len() {
            0 => unreachable!(),
            1 => 0,
            palette_count => {
                let packed_indices = PackedArray::from_inner(
                    self.data.as_ref().unwrap().as_ref().transmute(),
                    PackedArray::bits_per_entry(palette_count as u64 - 1).clamp(
                        *PALETTED_DATA_BLOCKS_INDIRECT.start() as u8,
                        *PALETTED_DATA_BLOCKS_INDIRECT.end() as u8,
                    ),
                    BLOCKS_PER_SECTION,
                );
                packed_indices.get_unchecked(index) as usize
            }
        }
    }

    fn get_block_by_index(&self, index: usize) -> Block {
        self.palette[self.get_block_palette_index_by_index(index)].clone()
    }

    fn get_block(&self, x: u8, y: u8, z: u8) -> Block {
        self.get_block_by_index(
            ((y as usize) & (SECTION_SIZE - 1)) * SECTION_SIZE * SECTION_SIZE
                + (z as usize) * SECTION_SIZE
                + (x as usize),
        )
    }

    fn blocks(&self) -> [Block; BLOCKS_PER_SECTION] {
        (0..BLOCKS_PER_SECTION)
            .map(|index| self.get_block_by_index(index))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    /// Returns none if any of the IDs are not found.
    fn blocks_ids(&self) -> Option<[i32; BLOCKS_PER_SECTION]> {
        let palette_ids = self
            .palette
            .iter()
            .map(|b| b.id())
            .collect::<Option<Box<[i32]>>>()?;
        Some(
            (0..BLOCKS_PER_SECTION)
                // TODO: Remove safety check when get_block_palette_index_by_index is fixed.
                .map(|index| palette_ids.get(self.get_block_palette_index_by_index(index)).cloned().unwrap_or(0))
                .collect::<Vec<i32>>()
                .try_into()
                .unwrap(),
        )
    }

    // NOTE: Data from this is already paletted correctly, All that's needed to do is convert to
    // IDs then send that into a packet, would be dramatically faster than what we're doing now.
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChunkSection {
    #[serde(rename = "Y")]
    y: i32,
    block_states: ChunkSectionBlockStates,
}

impl ChunkSection {
    pub fn get_block(&self, section_x: u8, section_y: u8, section_z: u8) -> Block {
        self.block_states.get_block(section_x, section_y, section_z)
    }

    pub fn blocks(&self) -> [Block; BLOCKS_PER_SECTION] {
        self.block_states.blocks()
    }

    pub fn blocks_ids(&self) -> Option<[i32; BLOCKS_PER_SECTION]> {
        self.block_states.blocks_ids()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
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
        let section_y = (y / SECTION_SIZE as i16) as i32;
        let Some(section) = self.sections.iter().find(|section| section.y == section_y) else {
            return Block::air();
        };
        section
            .block_states
            .get_block(x, (y & (SECTION_SIZE as i16 - 1)) as u8, z)
    }

    pub fn iter_sections(&self) -> impl Iterator<Item = &ChunkSection> + use<'_> {
        self.sections.iter()
    }
}

#[derive(Debug)]
#[allow(unused)]
struct Region {
    file: File,
    region_x: i32,
    region_z: i32,
    locations: [(u32, u32); CHUNKS_PER_REGION],
    loaded_chunks: HashMap<(u8, u8), Option<Chunk>>,
}

impl Region {
    fn load(mut file: File, region_x: i32, region_z: i32) -> Result<Self, WorldError> {
        let mut locations = [(0, 0); REGION_SIZE * REGION_SIZE];
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

    fn read(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<Box<[u8]>>, WorldError> {
        let (offset, length) =
            self.locations[(chunk_x as usize) + (chunk_z as usize) * REGION_SIZE];
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
        match self
            .read_nbt(chunk_x, chunk_z)?
            .map(|nbt| from_nbt::<Chunk>(nbt.1))
            .transpose()?
        {
            Some(chunk) if chunk.status == "minecraft:full" => Ok(Some(chunk)),
            _ => Ok(None),
        }
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
#[allow(unused)]
pub struct Level {
    identifier: String,
    root: PathBuf,
    loaded_regions: HashMap<(i32, i32), Option<Region>>,
}

impl Level {
    fn new(identifier: String, root: PathBuf) -> Self {
        Self {
            identifier,
            root,
            loaded_regions: HashMap::new(),
        }
    }

    fn load_region(&self, region_x: i32, region_z: i32) -> Result<Option<Region>, WorldError> {
        let mut path = self.root.clone();
        path.push("region");
        path.push(format!("r.{}.{}.mca", region_x, region_z));

        let file = match std::fs::File::open(path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            result => result,
        }?;

        Ok(Some(Region::load(file, region_x, region_z)?))
    }

    fn get_or_load_region(
        &mut self,
        region_x: i32,
        region_z: i32,
    ) -> Result<Option<&mut Region>, WorldError> {
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
        // FIXME: Use the const!
        let Some(region) = self.get_or_load_region(chunk_x >> 5, chunk_z >> 5)? else {
            //let Some(region) = self.get_or_load_region(chunk_x / REGION_SIZE as i32, chunk_z / REGION_SIZE as i32)? else {
            return Ok(None);
        };
        let Some(chunk) = region.get_or_load_chunk(
            (chunk_x & (REGION_SIZE - 1) as i32) as u8,
            (chunk_z & (REGION_SIZE - 1) as i32) as u8,
        )?
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
        let Some(chunk) =
            self.get_chunk(block_x / CHUNK_SIZE as i32, block_z / CHUNK_SIZE as i32)?
        else {
            return Ok(None);
        };
        let block = chunk.get_block(
            (block_x & (CHUNK_SIZE - 1) as i32) as u8,
            block_y,
            (block_z & (CHUNK_SIZE - 1) as i32) as u8,
        );
        Ok(Some(block))
    }
}

#[derive(Debug)]
#[allow(unused)]
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

    pub fn get_level(&mut self, identifier: &str) -> Option<&mut Level> {
        self.levels
            .iter_mut()
            .find(|level| level.identifier == identifier)
    }
}

#[cfg(test)]
mod test {
    use pkmc_defs::block::BLOCKS_TO_IDS;

    use crate::world::world::World;

    use super::WorldError;

    #[test]
    fn test_worldload() -> Result<(), WorldError> {
        // 1.21.4 debug world
        // https://minecraft.wiki/w/Debug_mode
        const WORLD_PATH: &str = "/home/vulae/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/pkmc/minecraft/saves/debug/";

        let mut world = World::load(WORLD_PATH)?;

        let level = world.get_level("minecraft:overworld").unwrap();

        let max_block_id = *BLOCKS_TO_IDS.values().max().unwrap();

        let start = std::time::Instant::now();
        println!("Total block ids: {}", max_block_id);

        let block_grid_width = (max_block_id as f32).sqrt().ceil() as i32;
        let _block_grid_height = (max_block_id as f32 / block_grid_width as f32).ceil() as i32;

        for block_id in 0..max_block_id {
            if block_id % 1024 == 0 {
                println!("Checking {} / {}", block_id, max_block_id);
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

        println!(
            "Checked all {} block ids in {:.2} seconds",
            max_block_id,
            std::time::Instant::now()
                .duration_since(start)
                .as_secs_f64()
        );

        Ok(())
    }
}
