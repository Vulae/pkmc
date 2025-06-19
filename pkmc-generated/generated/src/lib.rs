pub mod packet {
    use pkmc_generated_proc::report_packets_generate_consts;

    report_packets_generate_consts!("assets/reports/packets.json");

    pub const PROTOCOL_VERSION: i32 = 771;
}

pub mod registry {
    use pkmc_generated_proc::report_registry_generate_enum;

    report_registry_generate_enum!("assets/reports/registries.json", "minecraft:block_type", pub BlockType);

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
                    | BlockEntityType::EndGateway
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

    report_registry_generate_enum!("assets/reports/registries.json", "minecraft:command_argument_type", pub CommandArgumentType);
}

pub mod block {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::LazyLock,
    };

    use serde::Deserialize;

    use pkmc_generated_proc::{include_cached_json_compressed_bytes, report_blocks_enum};

    use crate::registry::{BlockEntityType, BlockType};

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

    pub trait IdIndexable {
        const NUM_STATES: u32;
        fn into_index(self) -> u32;
        fn from_index(index: u32) -> Option<Self>
        where
            Self: Sized;
    }

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
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
        const NUM_STATES: u32 = MAX + 1;

        fn into_index(self) -> u32 {
            self.0
        }

        fn from_index(index: u32) -> Option<Self> {
            (index <= MAX).then_some(PropertyUint(index))
        }
    }

    report_blocks_enum!("assets/reports/blocks.json", [
        Axis0 => Axis,
        Axis1 => AxisHor,
        Facing1 => Facing,
        Facing2 => FacingHor,
        Facing0 => FacingDownHor,
        Half1 => TopBottomHalf,
        Hinge0 => DoorHinge,
        Attachment0 => BellAttachment,
        Type2 => SlabType,
        Half0 => StairHalf,
        Shape2 => StairShape,
        North1 => WallShape,
        East1 => WallShape,
        South1 => WallShape,
        West1 => WallShape,
        Leaves0 => BambooLeaves,
        Tilt0 => DripleafTilt,
        Type1 => ChestType,
        Part0 => BedPart,
        Thickness0 => PointedDripstoneThickness,
        VerticalDirection0 => PointedDripstoneVerticalDirection,
        North2 => RedstoneWireShape,
        East2 => RedstoneWireShape,
        South2 => RedstoneWireShape,
        West2 => RedstoneWireShape,
        Mode0 => ComparatorMode,
        Face0 => ButtonFacing,
        Shape0 => NonturnableRailShape,
        Shape1 => TurnableRailShape,
        Type0 => PistonType,
        Instrument0 => Instrument,
        Orientation0 => Orientation,
        SculkSensorPhase => SculkSensorPhase,
        TrialSpawnerState0 => TrialSpawnerState,
        VaultState0 => VaultState,
        CreakingHeartState0 => CreakingHeartState,
        Mode1 => StructureBlockMode,
        Mode2 => TestBlockMode,
    ]);

    impl Default for Block {
        fn default() -> Self {
            Self::Air
        }
    }

    impl std::cmp::PartialEq for Block {
        fn eq(&self, other: &Self) -> bool {
            self.into_id() == other.into_id()
        }
    }

    impl std::cmp::Eq for Block {}

    impl std::cmp::Ord for Block {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.into_id().cmp(&other.into_id())
        }
    }

    impl std::cmp::PartialOrd for Block {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl std::hash::Hash for Block {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.into_id().hash(state);
        }
    }

    impl Block {
        pub const fn is_air(&self) -> bool {
            matches!(self, Block::Air | Block::CaveAir | Block::VoidAir)
        }

        pub const fn block_entity_type(&self) -> Option<BlockEntityType> {
            Some(match self.definition_type() {
                BlockType::Banner | BlockType::WallBanner => BlockEntityType::Banner,
                BlockType::Barrel => BlockEntityType::Barrel,
                BlockType::Beacon => BlockEntityType::Beacon,
                BlockType::Bed => BlockEntityType::Bed,
                BlockType::Beehive => BlockEntityType::Beehive,
                BlockType::Bell => BlockEntityType::Bell,
                BlockType::BlastFurnace => BlockEntityType::BlastFurnace,
                BlockType::BrewingStand => BlockEntityType::BrewingStand,
                BlockType::Brushable => BlockEntityType::BrushableBlock,
                BlockType::CalibratedSculkSensor => BlockEntityType::CalibratedSculkSensor,
                BlockType::Campfire => BlockEntityType::Campfire,
                BlockType::Chest => BlockEntityType::Chest,
                BlockType::ChiseledBookShelf => BlockEntityType::ChiseledBookshelf,
                BlockType::Command => BlockEntityType::CommandBlock,
                BlockType::Comparator => BlockEntityType::Comparator,
                BlockType::Conduit => BlockEntityType::Conduit,
                BlockType::Crafter => BlockEntityType::Crafter,
                BlockType::CreakingHeart => BlockEntityType::CreakingHeart,
                BlockType::DaylightDetector => BlockEntityType::DaylightDetector,
                BlockType::DecoratedPot => BlockEntityType::DecoratedPot,
                BlockType::Dispenser => BlockEntityType::Dispenser,
                BlockType::Dropper => BlockEntityType::Dropper,
                BlockType::EnchantmentTable => BlockEntityType::EnchantingTable,
                BlockType::EndGateway => BlockEntityType::EndGateway,
                BlockType::EndPortal => BlockEntityType::EndPortal,
                BlockType::EnderChest => BlockEntityType::EnderChest,
                BlockType::Furnace => BlockEntityType::Furnace,
                BlockType::CeilingHangingSign | BlockType::WallHangingSign => {
                    BlockEntityType::HangingSign
                }
                BlockType::Hopper => BlockEntityType::Hopper,
                BlockType::Jigsaw => BlockEntityType::Jigsaw,
                BlockType::Jukebox => BlockEntityType::Jukebox,
                BlockType::Lectern => BlockEntityType::Lectern,
                BlockType::Spawner => BlockEntityType::MobSpawner,
                BlockType::MovingPiston => BlockEntityType::Piston,
                BlockType::SculkCatalyst => BlockEntityType::SculkCatalyst,
                BlockType::SculkSensor => BlockEntityType::SculkSensor,
                BlockType::SculkShrieker => BlockEntityType::SculkShrieker,
                BlockType::ShulkerBox => BlockEntityType::ShulkerBox,
                BlockType::StandingSign | BlockType::WallSign => BlockEntityType::Sign,
                BlockType::PlayerHead | BlockType::PlayerWallHead => BlockEntityType::Skull,
                BlockType::Smoker => BlockEntityType::Smoker,
                BlockType::Structure => BlockEntityType::StructureBlock,
                BlockType::Test => BlockEntityType::TestBlock,
                BlockType::TestInstance => BlockEntityType::TestInstanceBlock,
                BlockType::TrappedChest => BlockEntityType::TrappedChest,
                BlockType::TrialSpawner => BlockEntityType::TrialSpawner,
                BlockType::Vault => BlockEntityType::Vault,
                _ => return None,
            })
        }
    }
}

pub mod consts {
    use std::ops::RangeInclusive;

    pub const VERSION_STR: &str = "1.21.6";

    pub const PALETTED_DATA_BLOCKS_INDIRECT: RangeInclusive<u8> = 4..=8;
    // TODO: Autogenerate this value.
    pub const PALETTED_DATA_BLOCKS_DIRECT: u8 = 15;
    pub const PALETTED_DATA_BIOMES_INDIRECT: RangeInclusive<u8> = 1..=3;
    // TODO: Autogenerate this value.
    pub const PALETTED_DATA_BIOMES_DIRECT: u8 = 6;
}

#[cfg(test)]
mod test_fake_blocks_report {
    use crate::{
        block::{IdIndexable, PropertyUint},
        registry::BlockType,
    };
    use pkmc_generated_proc::report_blocks_enum;

    report_blocks_enum!(
        "generated/src/test_blocks_report.json",
        [
            Facing0 => Facing,
        ]
    );

    impl std::cmp::PartialEq for Block {
        fn eq(&self, other: &Self) -> bool {
            self.into_id() == other.into_id()
        }
    }

    impl std::cmp::Eq for Block {}

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
    fn test_blocks_ids() {
        do_test(Block::Air, 0);
        do_test(Block::Stone, 1);
        do_test(Block::Barrier { waterlogged: true }, 2);
        do_test(Block::Barrier { waterlogged: false }, 3);
        do_test(Block::RedstoneWallTorch { facing: Facing::North, lit: true }, 4);
        do_test(Block::RedstoneWallTorch { facing: Facing::North, lit: false }, 5);
        do_test(Block::RedstoneWallTorch { facing: Facing::South, lit: true }, 6);
        do_test(Block::RedstoneWallTorch { facing: Facing::South, lit: false }, 7);
        do_test(Block::RedstoneWallTorch { facing: Facing::West, lit: true }, 8);
        do_test(Block::RedstoneWallTorch { facing: Facing::West, lit: false }, 9);
        do_test(Block::RedstoneWallTorch { facing: Facing::East, lit: true }, 10);
        do_test(Block::RedstoneWallTorch { facing: Facing::East, lit: false }, 11);
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
mod test {
    use crate::block::Block;

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
    fn test_blocks_ids() {
        do_test(Block::Air, 0);
        do_test(Block::Stone, 1);
    }
}
