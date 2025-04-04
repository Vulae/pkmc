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
}

pub mod block {
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::LazyLock,
    };

    use serde::Deserialize;

    use pkmc_generated_proc::include_cached_json_compressed_bytes;

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
