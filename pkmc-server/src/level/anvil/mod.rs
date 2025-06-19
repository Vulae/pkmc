use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use pkmc_defs::{biome::Biome, dimension::Dimension, registry::Registry};
use thiserror::Error;

use pkmc_util::{connection::ConnectionError, nbt::NBTError, IdTable};

mod chunk_format;
mod level;
mod region;

#[derive(Error, Debug)]
pub enum AnvilError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ConnectionError(#[from] ConnectionError),
    #[error("Region chunk unknown compression \"{0}\"")]
    RegionUnknownCompression(u8),
    #[error("Region chunk unsupported compression \"{0}\"")]
    RegionUnsupportedCompression(String),
    #[error(transparent)]
    NBTError(#[from] NBTError),
    #[error("Invalid block entity type \"{0}\"")]
    InvalidBlockEntityType(String),
}

pub use level::*;

use super::{Level, SECTION_BLOCKS_SIZE};

pub trait AnvilDimension {
    fn relative_directory(&self) -> Option<PathBuf>;
}

impl AnvilDimension for Dimension {
    fn relative_directory(&self) -> Option<PathBuf> {
        Some(match self.name() {
            "minecraft:overworld" => PathBuf::from("./region"),
            "minecraft:the_nether" => PathBuf::from("./DIM-1/region"),
            "minecraft:the_end" => PathBuf::from("./DIM1/region"),
            _ => return None,
        })
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct AnvilWorld {
    root: PathBuf,
    levels: HashMap<Dimension, Arc<Mutex<AnvilLevel>>>,
}

impl AnvilWorld {
    pub fn new<P: Into<PathBuf>>(
        root: P,
        biome_mapper: IdTable<Biome>,
        dimension_registry: &Registry,
    ) -> Result<Self, AnvilError> {
        let root: PathBuf = root.into();
        Ok(Self {
            root: root.clone(),
            levels: [
                Dimension::new("minecraft:overworld"),
                Dimension::new("minecraft:the_nether"),
                Dimension::new("minecraft:the_end"),
            ]
            .into_iter()
            .flat_map(|dimension| {
                let registry_entry = dimension_registry.get(dimension.name())?;
                let min_y = registry_entry.get("min_y")?.as_i64()?;
                let height = registry_entry.get("height")?.as_i64()?;
                Some((
                    dimension.clone(),
                    Arc::new(Mutex::new(AnvilLevel::new(
                        {
                            let mut path = root.clone();
                            path.push(dimension.relative_directory()?);
                            path.canonicalize().ok()?
                        },
                        dimension,
                        (min_y.div_euclid(SECTION_BLOCKS_SIZE as i64) as i8)
                            ..=(height.div_euclid(SECTION_BLOCKS_SIZE as i64) as i8),
                        biome_mapper.clone(),
                    ))),
                ))
            })
            .collect(),
        })
    }

    pub fn iter_levels(&self) -> impl Iterator<Item = (&Dimension, &Arc<Mutex<AnvilLevel>>)> {
        self.levels.iter()
    }

    pub fn level(&self, dimension: &Dimension) -> Option<Arc<Mutex<AnvilLevel>>> {
        self.levels.get(dimension).cloned()
    }

    pub fn update_viewers(&mut self) -> Result<(), AnvilError> {
        self.levels
            .values_mut()
            .try_for_each(|level| level.lock().unwrap().update_viewers())
    }
}

#[cfg(test)]
mod test {
    use pkmc_defs::{block::BLOCKS_TO_IDS, dimension::Dimension};
    use pkmc_util::Position;

    use crate::level::{anvil::AnvilWorld, Level as _};

    use super::AnvilError;

    #[test]
    fn test_debug_mode_world() -> Result<(), AnvilError> {
        // 1.21.6 debug world
        // https://minecraft.wiki/w/Debug_mode
        const WORLD_PATH: &str = "./src/level/anvil-test-server/world/";
        println!(
            "Testing debug world: {:?}",
            std::fs::canonicalize(WORLD_PATH)?
        );

        let world = AnvilWorld::new(
            WORLD_PATH,
            Default::default(),
            &[(
                "minecraft:overworld".to_owned(),
                serde_json::json!({
                    "min_y": -64,
                    "height": 384,
                }),
            )]
            .into_iter()
            .collect(),
        )?;
        let level_mutex = world.level(&Dimension::new("minecraft:overworld")).unwrap();
        let mut level = level_mutex.lock().unwrap();

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

            let position = Position::new(1 + grid_z * 2, 70, 1 + grid_x * 2);

            let Some(block) = level.get_block(position)? else {
                panic!("Expected loaded block at {:?}", position);
            };

            if block.into_id() != block_id {
                panic!(
                    "Block at {:?} is {:?} with ID {:?}, but our ID is {}",
                    position,
                    block,
                    block.into_id(),
                    block_id,
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
