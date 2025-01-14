use std::io::Write as _;

use pkmc_defs::{
    block::Block,
    generated::{
        generated, PALETTED_DATA_BIOMES_DIRECT, PALETTED_DATA_BIOMES_INDIRECT,
        PALETTED_DATA_BLOCKS_DIRECT, PALETTED_DATA_BLOCKS_INDIRECT,
    },
    packet,
};
use pkmc_util::{nbt_compound, packet::to_paletted_data};

pub mod anvil;
pub mod chunk_loader;

pub const CHUNK_SIZE: usize = 16;
pub const SECTION_SIZE: usize = 16;
pub const SECTION_BLOCKS: usize = 4096;

pub trait Chunk {
    fn get_block(&self, block_x: u8, block_y: i16, block_z: u8) -> Option<Block>;

    fn get_block_id(&self, block_x: u8, block_y: i16, block_z: u8) -> Option<i32> {
        self.get_block(block_x, block_y, block_z)
            .and_then(|b| b.id())
    }

    fn get_section_blocks(&self, section_y: i8) -> Option<[Block; SECTION_BLOCKS]>;

    fn get_section_blocks_ids(&self, section_y: i8) -> Option<[i32; SECTION_BLOCKS]> {
        self.get_section_blocks(section_y).and_then(|blocks| {
            blocks
                .iter()
                .map(|block| block.id_with_default_fallback())
                .collect::<Option<Vec<_>>>()
                .map(|inner| inner.try_into().unwrap())
        })
    }
}

pub trait World<C: Chunk> {
    type Error: std::error::Error + From<std::io::Error>;

    fn get_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Result<Option<&C>, Self::Error>;

    fn section_y_range(&self) -> std::ops::RangeInclusive<i8>;

    fn get_chunk_as_packet(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Option<packet::play::LevelChunkWithLight>, Self::Error> {
        let section_y_range = self.section_y_range();
        let Some(chunk) = self.get_chunk(chunk_x, chunk_z)? else {
            return Ok(None);
        };
        Ok(Some(packet::play::LevelChunkWithLight {
            chunk_x,
            chunk_z,
            chunk_data: packet::play::LevelChunkData {
                heightmaps: nbt_compound!(),
                data: {
                    let mut writer = Vec::new();

                    section_y_range.clone().try_for_each(|section_y| {
                        let block_ids = chunk
                            .get_section_blocks_ids(section_y)
                            .unwrap_or_else(|| [Block::air().id().unwrap(); SECTION_BLOCKS]);
                        // Num non-air blocks
                        let block_count = block_ids
                            .iter()
                            .filter(|b| !generated::block::is_air(**b))
                            .count();
                        writer.write_all(&(block_count as u16).to_be_bytes())?;
                        // Blocks
                        writer.write_all(&to_paletted_data(
                            &block_ids,
                            PALETTED_DATA_BLOCKS_INDIRECT,
                            PALETTED_DATA_BLOCKS_DIRECT,
                        )?)?;
                        // Biome
                        writer.write_all(&to_paletted_data(
                            &[0; 64],
                            PALETTED_DATA_BIOMES_INDIRECT,
                            PALETTED_DATA_BIOMES_DIRECT,
                        )?)?;
                        Ok(())
                    })?;

                    writer.into_boxed_slice()
                },
                // TODO: Block entities
                block_entities: Vec::new(),
            },
            // TODO: Light data
            light_data: packet::play::LevelLightData::full_dark(self.section_y_range().count()),
        }))
    }

    fn get_block(
        &mut self,
        block_x: i32,
        block_y: i16,
        block_z: i32,
    ) -> Result<Option<Block>, Self::Error> {
        let Some(chunk) = self.get_chunk(
            block_x.div_euclid(CHUNK_SIZE as i32),
            block_z.div_euclid(CHUNK_SIZE as i32),
        )?
        else {
            return Ok(None);
        };
        Ok(chunk.get_block(
            (block_x % (CHUNK_SIZE as i32)) as u8,
            block_y,
            (block_z % (CHUNK_SIZE as i32)) as u8,
        ))
    }
}
