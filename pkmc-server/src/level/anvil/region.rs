use std::{collections::HashMap, fs::File, io::Seek as _, path::PathBuf};

use pkmc_util::{nbt::NBT, ReadExt as _};

use super::AnvilError;

pub const REGION_SIZE: usize = 32;
pub const CHUNKS_PER_REGION: usize = REGION_SIZE * REGION_SIZE;

#[derive(Debug)]
pub(super) struct Region {
    file: File,
    locations: [(u32, u32); CHUNKS_PER_REGION],
}

impl Region {
    fn load(mut file: File) -> Result<Option<Self>, AnvilError> {
        file.rewind()?;
        let raw: [u8; 8 * CHUNKS_PER_REGION] = match file.read_const() {
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(err) => Err(err)?,
            Ok(raw) => raw,
        };
        Ok(Some(Self {
            file,
            locations: std::array::from_fn(|i| {
                let offset = ((raw[i * 4 + 2] as u32)
                    | ((raw[i * 4 + 1] as u32) << 8)
                    | ((raw[i * 4] as u32) << 16))
                    * 0x1000;
                let length = (raw[i * 4 + 3] as u32) * 0x1000;
                (offset, length)
            }),
        }))
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
}

pub(super) trait ChunkParser {
    type Chunk;
    fn parse(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        nbt: (String, NBT),
    ) -> Result<Option<Self::Chunk>, AnvilError>;
}

#[derive(Debug)]
pub(super) struct ChunkReader<Parser: ChunkParser> {
    directory: PathBuf,
    regions: HashMap<(i32, i32), Option<Region>>,
    parser: Parser,
    chunks: HashMap<(i32, i32), Option<Parser::Chunk>>,
}

impl<Parser: ChunkParser> ChunkReader<Parser> {
    pub fn new(directory: PathBuf, parser: Parser) -> Self {
        Self {
            directory,
            regions: HashMap::new(),
            parser,
            chunks: HashMap::new(),
        }
    }

    fn prepare_region(&mut self, region_x: i32, region_z: i32) -> Result<(), AnvilError> {
        if self.regions.contains_key(&(region_x, region_z)) {
            return Ok(());
        }
        self.regions.insert((region_x, region_z), None);

        let mut path = self.directory.clone();
        path.push(format!("r.{}.{}.mca", region_x, region_z));

        let file = match std::fs::File::open(path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(());
            }
            result => result,
        }?;

        self.regions
            .insert((region_x, region_z), Region::load(file)?);

        Ok(())
    }

    /// When getting a chunk that may or may not already be loaded, call this before getting the
    /// chunk.
    pub fn prepare_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Result<(), AnvilError> {
        if self.chunks.contains_key(&(chunk_x, chunk_z)) {
            return Ok(());
        }
        self.chunks.insert((chunk_x, chunk_z), None);

        let region_x = chunk_x.div_euclid(REGION_SIZE as i32);
        let region_z = chunk_z.div_euclid(REGION_SIZE as i32);

        self.prepare_region(region_x, region_z)?;

        let Some(Some(region)) = self.regions.get_mut(&(region_x, region_z)) else {
            return Ok(());
        };

        let Some(nbt) = region.read_nbt(
            chunk_x.rem_euclid(REGION_SIZE as i32) as u8,
            chunk_z.rem_euclid(REGION_SIZE as i32) as u8,
        )?
        else {
            return Ok(());
        };

        let Some(chunk) = self.parser.parse(chunk_x, chunk_z, nbt)? else {
            return Ok(());
        };

        self.chunks.insert((chunk_x, chunk_z), Some(chunk));

        Ok(())
    }

    /// If the chunk may or may not already be loaded, call [`ChunkReader::prepare_chunk`] first.
    pub fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&Parser::Chunk> {
        self.chunks
            .get(&(chunk_x, chunk_z))
            .and_then(|i| i.as_ref())
    }

    /// If the chunk may or may not already be loaded, call [`ChunkReader::prepare_chunk`] first.
    pub fn get_mut_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut Parser::Chunk> {
        self.chunks
            .get_mut(&(chunk_x, chunk_z))
            .and_then(|i| i.as_mut())
    }
}
