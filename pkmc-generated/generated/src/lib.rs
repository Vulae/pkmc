pub mod packet {
    use pkmc_generated_proc::report_packets_generate_consts;

    report_packets_generate_consts!("assets/reports/packets.json");

    pub const PROTOCOL_VERSION: i32 = 770;
}

pub mod registry {
    use pkmc_generated_proc::report_registry_generate_enum;

    report_registry_generate_enum!("assets/reports/registries.json", "minecraft:block_entity_type", pub BlockEntityType);
    impl BlockEntityType {
        /// If NBT contents of this block entity type reflect a visible change in the block.
        pub const fn nbt_visible(&self) -> bool {
            // Note that some block entity types (like beehive) do have visible changes, but that is
            // in the normal block state instead, so this returns false for those.
            matches!(
                self,
                BlockEntityType::Sign // Required to render sign text
                    | BlockEntityType::HangingSign // Required to render sign text
                    | BlockEntityType::Skull // Required to render mob heads & custom heads
                    | BlockEntityType::Banner // Required to render banner pattern
                    | BlockEntityType::DecoratedPot // Required to render sherds on sides
                    | BlockEntityType::Campfire // Required to render cooking items
                    | BlockEntityType::MobSpawner // Required to render containing entity
                    | BlockEntityType::TrialSpawner // Required to render containing entity
                    | BlockEntityType::Vault // Required to render containing item
                                             //| BlockEntityType::Piston // ???
                                             //| BlockEntityType::SuspiciousGravel // Doesn't exist???
                                             //| BlockEntityType::SuspiciousSand // Doesn't exist???
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
