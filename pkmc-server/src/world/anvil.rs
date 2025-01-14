use std::{collections::HashMap, fs::File, io::Seek, path::PathBuf};

use pkmc_defs::{block::Block, generated::PALETTED_DATA_BLOCKS_INDIRECT};
use pkmc_util::{
    nbt::{from_nbt, NBTError, NBT},
    PackedArray, ReadExt, Transmutable,
};
use serde::Deserialize;
use thiserror::Error;

use crate::world::SECTION_SIZE;

use super::{Chunk, World, SECTION_BLOCKS};

pub const REGION_SIZE: usize = 32;
pub const CHUNKS_PER_REGION: usize = REGION_SIZE * REGION_SIZE;

#[derive(Error, Debug)]
pub enum AnvilError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("Region chunk unknown compression \"{0}\"")]
    RegionUnknownCompression(u8),
    #[error("Region chunk unsupported compression \"{0}\"")]
    RegionUnsupportedCompression(String),
    #[error(transparent)]
    NBTError(#[from] NBTError),
}

fn default_blocks_palette() -> Box<[Block]> {
    vec![Block::air()].into_boxed_slice()
}

#[derive(Debug, Deserialize, Clone)]
struct ChunkSectionBlockStates {
    #[serde(default = "default_blocks_palette")]
    palette: Box<[Block]>,
    #[serde(default)]
    data: Option<Box<[i64]>>,
    #[serde(skip, default)]
    palette_ids: Box<[i32]>,
}

impl ChunkSectionBlockStates {
    fn initialize(&mut self) {
        self.palette_ids = self
            .palette
            .iter()
            .map(|b| {
                b.id().unwrap_or_else(|| {
                    b.without_properties()
                        .id()
                        .unwrap_or(Block::air().id().unwrap())
                })
            })
            .collect();
    }

    #[inline(always)]
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
                    SECTION_BLOCKS,
                );
                packed_indices.get_unchecked(index) as usize
            }
        }
    }

    fn get_block_by_index(&self, index: usize) -> Block {
        self.palette
            .get(self.get_block_palette_index_by_index(index))
            .cloned()
            .unwrap()
    }

    fn get_block(&self, x: u8, y: u8, z: u8) -> Block {
        debug_assert!((x as usize) < SECTION_SIZE);
        debug_assert!((y as usize) < SECTION_SIZE);
        debug_assert!((z as usize) < SECTION_SIZE);
        self.get_block_by_index(
            (y as usize) * SECTION_SIZE * SECTION_SIZE + (z as usize) * SECTION_SIZE + (x as usize),
        )
    }

    fn blocks(&self) -> [Block; SECTION_BLOCKS] {
        (0..SECTION_BLOCKS)
            .map(|index| self.get_block_by_index(index))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    /// Returns none if any of the IDs are not found.
    fn blocks_ids(&self) -> [i32; SECTION_BLOCKS] {
        (0..SECTION_BLOCKS)
            .map(|index| {
                *self
                    .palette_ids
                    .get(self.get_block_palette_index_by_index(index))
                    .unwrap()
            })
            .collect::<Vec<i32>>()
            .try_into()
            .unwrap()
    }

    // NOTE: Data from this is already paletted correctly, All that's needed to do is convert to
    // IDs then send that into a packet, would be dramatically faster if we included an option to
    // just directly convert to packet data.
}

#[derive(Debug, Deserialize, Clone)]
struct ChunkSection {
    #[serde(rename = "Y")]
    y: i8,
    block_states: Option<ChunkSectionBlockStates>,
}

impl ChunkSection {
    fn initialize(&mut self) {
        if let Some(ref mut block_states) = self.block_states {
            block_states.initialize();
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnvilChunk {
    //#[serde(rename = "DataVersion")]
    //data_version: i32,
    //#[serde(rename = "xPos")]
    //x_pos: i32,
    //#[serde(rename = "zPos")]
    //z_pos: i32,
    //#[serde(rename = "yPos")]
    //y_pos: i32,
    //#[serde(rename = "Status")]
    //status: String,
    //#[serde(rename = "LastUpdate")]
    //last_update: i64,
    sections: Vec<ChunkSection>,
}

impl AnvilChunk {
    fn initialize(&mut self) {
        // Sometimes sections are unsorted.
        self.sections.sort_by(|a, b| a.y.cmp(&b.y));
        self.sections
            .iter_mut()
            .for_each(|section| section.initialize());
    }

    fn get_section(&self, section_y: i8) -> Option<&ChunkSection> {
        self.sections.iter().find(|section| section.y == section_y)
    }
}

impl Chunk for AnvilChunk {
    fn get_block(&self, block_x: u8, block_y: i16, block_z: u8) -> Option<Block> {
        debug_assert!((block_x as usize) < SECTION_SIZE);
        debug_assert!((block_z as usize) < SECTION_SIZE);
        Some(
            self.get_section((block_y / SECTION_SIZE as i16) as i8)?
                .block_states
                .as_ref()?
                .get_block(
                    block_x,
                    (block_y.rem_euclid(SECTION_SIZE as i16)) as u8,
                    block_z,
                ),
        )
    }

    fn get_section_blocks(&self, section_y: i8) -> Option<[Block; SECTION_BLOCKS]> {
        Some(self.get_section(section_y)?.block_states.as_ref()?.blocks())
    }

    fn get_section_blocks_ids(&self, section_y: i8) -> Option<[i32; SECTION_BLOCKS]> {
        Some(
            self.get_section(section_y)?
                .block_states
                .as_ref()?
                .blocks_ids(),
        )
    }
}

#[derive(Debug)]
#[allow(unused)]
struct Region {
    file: File,
    region_x: i32,
    region_z: i32,
    locations: [(u32, u32); CHUNKS_PER_REGION],
    loaded_chunks: HashMap<(u8, u8), Option<AnvilChunk>>,
}

impl Region {
    fn load(mut file: File, region_x: i32, region_z: i32) -> Result<Self, AnvilError> {
        let mut locations = [(0, 0); REGION_SIZE * REGION_SIZE];
        file.rewind()?;
        locations.iter_mut().try_for_each(|(offset, length)| {
            let data = u32::from_be_bytes(file.read_const()?);
            *offset = ((data & 0xFFFFFF00) >> 8) * 0x1000;
            *length = (data & 0x000000FF) * 0x1000;
            Ok::<_, AnvilError>(())
        })?;
        Ok(Self {
            file,
            region_x,
            region_z,
            locations,
            loaded_chunks: HashMap::new(),
        })
    }

    fn read(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<Box<[u8]>>, AnvilError> {
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
            1 => Err(AnvilError::RegionUnsupportedCompression("GZip".to_owned())),
            2 => Ok(Some(
                flate2::read::ZlibDecoder::new(std::io::Cursor::new(compressed_data)).read_all()?,
            )),
            3 => Ok(Some(compressed_data)),
            4 => Err(AnvilError::RegionUnsupportedCompression("LZ4".to_owned())),
            127 => {
                let mut data = std::io::Cursor::new(&compressed_data);
                let string_length = u16::from_be_bytes(data.read_const()?);
                let string_buf = data.read_var(string_length as usize)?;
                Err(AnvilError::RegionUnsupportedCompression(format!(
                    "Custom {}",
                    String::from_utf8_lossy(&string_buf)
                )))
            }
            _ => Err(AnvilError::RegionUnknownCompression(compression_type)),
        }
    }

    fn read_nbt(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<(String, NBT)>, AnvilError> {
        Ok(self
            .read(chunk_x, chunk_z)?
            .map(|data| NBT::read(std::io::Cursor::new(data), false))
            .transpose()?)
    }

    fn load_chunk(&mut self, chunk_x: u8, chunk_z: u8) -> Result<Option<AnvilChunk>, AnvilError> {
        match self
            .read_nbt(chunk_x, chunk_z)?
            .map(|nbt| from_nbt::<AnvilChunk>(nbt.1))
            .transpose()?
        {
            //Some(chunk) if chunk.status == "minecraft:full" => Ok(Some(chunk)),
            Some(mut chunk) => {
                chunk.initialize();
                Ok(Some(chunk))
            }
            _ => Ok(None),
        }
    }

    fn get_or_load_chunk(
        &mut self,
        chunk_x: u8,
        chunk_z: u8,
    ) -> Result<Option<&AnvilChunk>, AnvilError> {
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
pub struct AnvilWorld {
    root: PathBuf,
    loaded_regions: HashMap<(i32, i32), Option<Region>>,
    section_y_range: std::ops::RangeInclusive<i8>,
}

impl AnvilWorld {
    pub fn new<P: Into<PathBuf>>(root: P, section_y_range: std::ops::RangeInclusive<i8>) -> Self {
        Self {
            root: root.into(),
            loaded_regions: HashMap::new(),
            section_y_range,
        }
    }

    fn load_region(&self, region_x: i32, region_z: i32) -> Result<Option<Region>, AnvilError> {
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
    ) -> Result<Option<&mut Region>, AnvilError> {
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

    fn get_chunk_inner(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Option<&AnvilChunk>, AnvilError> {
        // FIXME: Use the const!
        let Some(region) = self.get_or_load_region(
            chunk_x.div_euclid(REGION_SIZE as i32),
            chunk_z.div_euclid(REGION_SIZE as i32),
        )?
        else {
            return Ok(None);
        };
        let Some(chunk) = region.get_or_load_chunk(
            (chunk_x.wrapping_rem_euclid(REGION_SIZE as i32)) as u8,
            (chunk_z.wrapping_rem_euclid(REGION_SIZE as i32)) as u8,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(chunk))
    }
}

impl World<AnvilChunk> for AnvilWorld {
    type Error = AnvilError;

    fn section_y_range(&self) -> std::ops::RangeInclusive<i8> {
        self.section_y_range.clone()
    }

    fn get_chunk(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Option<&AnvilChunk>, Self::Error> {
        self.get_chunk_inner(chunk_x, chunk_z)
    }
}

#[cfg(test)]
mod test {
    use pkmc_defs::block::BLOCKS_TO_IDS;

    use crate::world::{anvil::AnvilWorld, World as _};

    use super::AnvilError;

    #[test]
    fn test_worldload() -> Result<(), AnvilError> {
        // 1.21.4 debug world
        // https://minecraft.wiki/w/Debug_mode
        const WORLD_PATH: &str = "/home/vulae/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/pkmc/minecraft/saves/debug/";

        let mut world = AnvilWorld::new(WORLD_PATH);

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

            let Some(block) = world.get_block(x, y, z)? else {
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
