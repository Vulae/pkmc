pub mod packet {
    use pkmc_generated_proc::report_packets_generate_consts;

    report_packets_generate_consts!("assets/reports/packets.json");

    pub const PROTOCOL_VERSION: i32 = 770;
}

pub mod registry {
    use pkmc_generated_proc::report_registry_generate_enum;

    report_registry_generate_enum!("assets/reports/registries.json", "minecraft:block_entity_type", pub BlockEntityType);
    impl BlockEntityType {
        /// If NBT contents is required to render this block, or NBT contents reflect a visible change in the block.
        pub const fn nbt_visible(&self) -> bool {
            matches!(
                self,
                BlockEntityType::Banner
                    | BlockEntityType::Beacon
                    | BlockEntityType::Bed
                    | BlockEntityType::Bell
                    | BlockEntityType::BrewingStand
                    | BlockEntityType::BrushableBlock
                    | BlockEntityType::Campfire
                    | BlockEntityType::Chest
                    | BlockEntityType::ChiseledBookshelf
                    | BlockEntityType::Conduit
                    | BlockEntityType::DecoratedPot
                    // | BlockEntityType::EndGateway
                    | BlockEntityType::EndPortal
                    | BlockEntityType::EnderChest
                    | BlockEntityType::HangingSign
                    | BlockEntityType::MobSpawner
                    | BlockEntityType::Piston
                    | BlockEntityType::ShulkerBox
                    | BlockEntityType::Sign
                    | BlockEntityType::Skull
                    | BlockEntityType::TrappedChest
                    | BlockEntityType::TrialSpawner
            )
        }
    }

    report_registry_generate_enum!("assets/reports/registries.json", "minecraft:entity_type", pub EntityType);
    report_registry_generate_enum!("assets/reports/registries.json", "minecraft:particle_type", pub ParticleType);
}

pub mod block {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::LazyLock,
    };

    use serde::Deserialize;

    use pkmc_generated_proc::{include_cached_json_compressed_bytes, report_blocks_enum};

    #[derive(Deserialize)]
    pub struct ReportBlockState {
        pub id: i32,
        #[serde(default)]
        pub default: bool,
        #[serde(default)]
        pub properties: BTreeMap<String, String>,
    }

    #[derive(Deserialize)]
    pub struct ReportBlock {
        pub definition: serde_json::Value,
        #[serde(default)]
        pub properties: BTreeMap<String, BTreeSet<String>>,
        pub states: Vec<ReportBlockState>,
    }

    #[derive(Deserialize)]
    pub struct ReportBlocks(pub BTreeMap<String, ReportBlock>);

    pub static BLOCKS_REPORT: LazyLock<ReportBlocks> = LazyLock::new(|| {
        serde_json::from_slice(&include_cached_json_compressed_bytes!(
            "assets/reports/blocks.json",
            "assets/reports/blocks.json.gz"
        ))
        .unwrap()
    });

    // TODO: Autogenerate this
    pub const fn is_air(id: i32) -> bool {
        matches!(id, 0 | 13971 | 13972)
    }

    pub trait IdIndexable {
        const MAX_INDEX: u32;
        fn into_index(self) -> u32;
        fn from_index(index: u32) -> Option<Self>
        where
            Self: Sized;
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PropertyUint<const MAX: u32>(u32);

    impl<const MAX: u32> PropertyUint<MAX> {
        /// Asserts if outside of range, use <PropertyUint<MAX> as IdIndexable>::from_index for runtime creation instead.
        pub const fn new(value: u32) -> Self {
            // TODO: Make compile-time check.
            assert!(value <= MAX, "PropertyUint value is outside of range");
            Self(value)
        }
    }

    impl<const MAX: u32> IdIndexable for PropertyUint<MAX> {
        const MAX_INDEX: u32 = MAX;

        fn into_index(self) -> u32 {
            self.0
        }

        fn from_index(index: u32) -> Option<Self> {
            (index <= MAX).then_some(PropertyUint(index))
        }
    }

    report_blocks_enum!("assets/reports/blocks.json");

    impl Default for Block {
        fn default() -> Self {
            Self::Air
        }
    }

    impl Block {
        pub fn is_air(&self) -> bool {
            matches!(self, Block::Air | Block::CaveAir | Block::VoidAir)
        }
    }
}

pub mod consts {
    use std::ops::RangeInclusive;

    pub const VERSION_STR: &str = "1.21.5";

    pub const PALETTED_DATA_BLOCKS_INDIRECT: RangeInclusive<u8> = 4..=8;
    // TODO: Autogenerate this value.
    pub const PALETTED_DATA_BLOCKS_DIRECT: u8 = 15;
    pub const PALETTED_DATA_BIOMES_INDIRECT: RangeInclusive<u8> = 1..=3;
    // TODO: Autogenerate this value.
    pub const PALETTED_DATA_BIOMES_DIRECT: u8 = 6;
}

#[cfg(test)]
mod simple_test {
    use crate::block::{IdIndexable, PropertyUint};
    use pkmc_generated_proc::report_blocks_enum;

    report_blocks_enum!("generated/src/test_blocks_report.json");

    fn do_test(block: Block, id: i32) {
        if id != block.into_id() {
            panic!(
                "Block::into_id test failed for block {:?}, expected {} but got {}",
                block,
                id,
                block.into_id(),
            );
        }
        if Some(block) != Block::from_id(id) {
            panic!(
                "Block::from_id test failed for id {}, expected {:?} but got {:?}",
                id,
                Some(block),
                Block::from_id(id),
            );
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_simple_blocks_ids() {
        do_test(Block::Air, 0);
        do_test(Block::Stone, 1);
        do_test(Block::Barrier { waterlogged: true }, 2);
        do_test(Block::Barrier { waterlogged: false }, 3);
        do_test(Block::RedstoneWallTorch { facing: Facing0::North, lit: true }, 4);
        do_test(Block::RedstoneWallTorch { facing: Facing0::North, lit: false }, 5);
        do_test(Block::RedstoneWallTorch { facing: Facing0::South, lit: true }, 6);
        do_test(Block::RedstoneWallTorch { facing: Facing0::South, lit: false }, 7);
        do_test(Block::RedstoneWallTorch { facing: Facing0::West, lit: true }, 8);
        do_test(Block::RedstoneWallTorch { facing: Facing0::West, lit: false }, 9);
        do_test(Block::RedstoneWallTorch { facing: Facing0::East, lit: true }, 10);
        do_test(Block::RedstoneWallTorch { facing: Facing0::East, lit: false }, 11);
        do_test(Block::Wheat { age: PropertyUint::new(0) }, 12);
        do_test(Block::Wheat { age: PropertyUint::new(1) }, 13);
        do_test(Block::Wheat { age: PropertyUint::new(2) }, 14);
        do_test(Block::Wheat { age: PropertyUint::new(3) }, 15);
        do_test(Block::Wheat { age: PropertyUint::new(4) }, 16);
        do_test(Block::Wheat { age: PropertyUint::new(5) }, 17);
        do_test(Block::Wheat { age: PropertyUint::new(6) }, 18);
        do_test(Block::Wheat { age: PropertyUint::new(7) }, 19);
    }
}

#[cfg(test)]
mod complex_test {
    use crate::block::{self, Block, PropertyUint};

    fn do_test(block: Block, id: i32) {
        if id != block.into_id() {
            panic!(
                "Block::into_id test failed for block {:?}, expected {} but got {}",
                block,
                id,
                block.into_id(),
            );
        }
        if Some(block) != Block::from_id(id) {
            panic!(
                "Block::from_id test failed for id {}, expected {:?} but got {:?}",
                id,
                Some(block),
                Block::from_id(id),
            );
        }
    }

    #[test]
    fn test_simple_blocks_ids() {
        do_test(Block::Air, 0);
        do_test(Block::Stone, 1);
        // Just some random block states to test, not anything special to them.
        do_test(
            Block::RedStainedGlassPane {
                east: false,
                north: false,
                south: true,
                waterlogged: false,
                west: false,
            },
            10656,
        );
        do_test(
            Block::RedstoneWire {
                east: block::East2::Up,
                north: block::North2::Up,
                power: PropertyUint::new(14),
                south: block::South2::Up,
                west: block::West2::None,
            },
            3170,
        );
    }
}
