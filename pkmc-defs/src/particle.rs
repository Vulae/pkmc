use std::sync::LazyLock;

use pkmc_util::{Color, IdTable, Position, Vec3};

use crate::{block::Block, generated::DATA};

#[derive(Debug)]
pub enum Particle {
    AngryVillager,
    Block(Block),
    BlockMarker(Block),
    Bubble,
    Cloud,
    Crit,
    DamageIndicator,
    DragonBreath,
    DrippingLava,
    FallingLava,
    LandingLava,
    DrippingWater,
    FallingWater,
    Dust {
        color: Color,
        scale: f32,
    },
    DustColorTransition {
        from: Color,
        to: Color,
        scale: f32,
    },
    Effect,
    ElderGuardian,
    Enchant,
    EndRod,
    EntityEffect {
        color: Color,
        alpha: u8,
    },
    ExplosionEmitter,
    Explosion,
    Gust,
    SmallGust,
    GustEmitterLarge,
    GustEmitterSmall,
    SonicBoom,
    FallingDust(Block),
    Firework,
    Fishing,
    Flame,
    Infested,
    CherryLeaves,
    PaleOakLeaves,
    SculkSoul,
    SculkCharge {
        roll: f32,
    },
    SculkChargePop,
    SoulFireFlame,
    Soul,
    Flash,
    HappyVillager,
    Composter,
    Heart,
    InstantEffect,
    // TODO: Implete slot data
    // https://minecraft.wiki/w/Minecraft_Wiki:Projects/wiki.vg_merge/Slot_Data
    Item,
    Vibration {
        source: VibrationSource,
        ticks: i32,
    },
    Trail {
        position: Vec3<f64>,
        color: Color,
        duration: i32,
    },
    ItemSlime,
    ItemCobweb,
    ItemSnowball,
    LargeSmoke,
    Lava,
    Mycelium,
    Note,
    Poof,
    Portal,
    Rain,
    Smoke,
    WhiteSmoke,
    Sneeze,
    Spit,
    SquidInk,
    SweepAttack,
    TotemOfUndying,
    Underwater,
    Splash,
    Witch,
    BubblePop,
    CurrentDown,
    BubbleColumnUp,
    Nautilus,
    Dolphin,
    CampfireCosySmoke,
    CampfireSignalSmoke,
    DrippingHoney,
    FallingHoney,
    LandingHoney,
    FallingNectar,
    FallingSporeBlossom,
    Ash,
    CrimsonSpore,
    WarpedSpore,
    SporeBlossomAir,
    DrippingObsidianTear,
    FallingObsidianTear,
    LandingObsidianTear,
    ReversePortal,
    WhiteAsh,
    SmallFlame,
    Snowflake,
    DrippingDripstoneLava,
    FallingDripstoneLava,
    DrippingDripstoneWater,
    FallingDripstoneWater,
    GlowSquidInk,
    Glow,
    WaxOn,
    WaxOff,
    ElectricSpark,
    Scrape,
    Shriek {
        delay: i32,
    },
    EggCrack,
    DustPlume,
    TrialSpawnerDetection,
    TrailSpawnerDetectionOminous,
    VaultConnection,
    DustPillar(Block),
    OminousSpawning,
    RaidOmen,
    TrailOmen,
    BlockCrumble(Block),
}

#[derive(Debug)]
pub enum VibrationSource {
    Block(Position),
    Entity { id: i32, eye_height: f32 },
}

impl Particle {
    pub(crate) fn id(&self) -> i32 {
        match self {
            Particle::AngryVillager => *PARTICLES_TO_IDS.get("minecraft:angry_villager").unwrap(),
            Particle::Block(..) => *PARTICLES_TO_IDS.get("minecraft:block").unwrap(),
            Particle::BlockMarker(..) => *PARTICLES_TO_IDS.get("minecraft:block_marker").unwrap(),
            Particle::Bubble => *PARTICLES_TO_IDS.get("minecraft:bubble").unwrap(),
            Particle::Cloud => *PARTICLES_TO_IDS.get("minecraft:cloud").unwrap(),
            Particle::Crit => *PARTICLES_TO_IDS.get("minecraft:crit").unwrap(),
            Particle::DamageIndicator => {
                *PARTICLES_TO_IDS.get("minecraft:damage_indicator").unwrap()
            }
            Particle::DragonBreath => *PARTICLES_TO_IDS.get("minecraft:dragon_breath").unwrap(),
            Particle::DrippingLava => *PARTICLES_TO_IDS.get("minecraft:dripping_lava").unwrap(),
            Particle::FallingLava => *PARTICLES_TO_IDS.get("minecraft:falling_lava").unwrap(),
            Particle::LandingLava => *PARTICLES_TO_IDS.get("minecraft:landing_lava").unwrap(),
            Particle::DrippingWater => *PARTICLES_TO_IDS.get("minecraft:dripping_water").unwrap(),
            Particle::FallingWater => *PARTICLES_TO_IDS.get("minecraft:falling_water").unwrap(),
            Particle::Dust { .. } => *PARTICLES_TO_IDS.get("minecraft:dust").unwrap(),
            Particle::DustColorTransition { .. } => *PARTICLES_TO_IDS
                .get("minecraft:dust_color_transition")
                .unwrap(),
            Particle::Effect => *PARTICLES_TO_IDS.get("minecraft:effect").unwrap(),
            Particle::ElderGuardian => *PARTICLES_TO_IDS.get("minecraft:elder_guardian").unwrap(),
            Particle::Enchant => *PARTICLES_TO_IDS.get("minecraft:enchant").unwrap(),
            Particle::EndRod => *PARTICLES_TO_IDS.get("minecraft:end_rod").unwrap(),
            Particle::EntityEffect { .. } => {
                *PARTICLES_TO_IDS.get("minecraft:entity_effect").unwrap()
            }
            Particle::ExplosionEmitter => {
                *PARTICLES_TO_IDS.get("minecraft:explosion_emitter").unwrap()
            }
            Particle::Explosion => *PARTICLES_TO_IDS.get("minecraft:explosion").unwrap(),
            Particle::Gust => *PARTICLES_TO_IDS.get("minecraft:gust").unwrap(),
            Particle::SmallGust => *PARTICLES_TO_IDS.get("minecraft:small_gust").unwrap(),
            Particle::GustEmitterLarge => *PARTICLES_TO_IDS
                .get("minecraft:gust_emitter_large")
                .unwrap(),
            Particle::GustEmitterSmall => *PARTICLES_TO_IDS
                .get("minecraft:gust_emitter_small")
                .unwrap(),
            Particle::SonicBoom => *PARTICLES_TO_IDS.get("minecraft:sonic_boom").unwrap(),
            Particle::FallingDust(_) => *PARTICLES_TO_IDS.get("minecraft:falling_dust").unwrap(),
            Particle::Firework => *PARTICLES_TO_IDS.get("minecraft:firework").unwrap(),
            Particle::Fishing => *PARTICLES_TO_IDS.get("minecraft:fishing").unwrap(),
            Particle::Flame => *PARTICLES_TO_IDS.get("minecraft:flame").unwrap(),
            Particle::Infested => *PARTICLES_TO_IDS.get("minecraft:infested").unwrap(),
            Particle::CherryLeaves => *PARTICLES_TO_IDS.get("minecraft:cherry_leaves").unwrap(),
            Particle::PaleOakLeaves => *PARTICLES_TO_IDS.get("minecraft:pale_oak_leaves").unwrap(),
            Particle::SculkSoul => *PARTICLES_TO_IDS.get("minecraft:sculk_soul").unwrap(),
            Particle::SculkCharge { .. } => {
                *PARTICLES_TO_IDS.get("minecraft:sculk_charge").unwrap()
            }
            Particle::SculkChargePop => {
                *PARTICLES_TO_IDS.get("minecraft:sculk_charge_pop").unwrap()
            }
            Particle::SoulFireFlame => *PARTICLES_TO_IDS.get("minecraft:soul_fire_flame").unwrap(),
            Particle::Soul => *PARTICLES_TO_IDS.get("minecraft:soul").unwrap(),
            Particle::Flash => *PARTICLES_TO_IDS.get("minecraft:flash").unwrap(),
            Particle::HappyVillager => *PARTICLES_TO_IDS.get("minecraft:happy_villager").unwrap(),
            Particle::Composter => *PARTICLES_TO_IDS.get("minecraft:composter").unwrap(),
            Particle::Heart => *PARTICLES_TO_IDS.get("minecraft:heart").unwrap(),
            Particle::InstantEffect => *PARTICLES_TO_IDS.get("minecraft:instant_effect").unwrap(),
            Particle::Item => *PARTICLES_TO_IDS.get("minecraft:item").unwrap(),
            Particle::Vibration { .. } => *PARTICLES_TO_IDS.get("minecraft:vibration").unwrap(),
            Particle::Trail { .. } => *PARTICLES_TO_IDS.get("minecraft:trail").unwrap(),
            Particle::ItemSlime => *PARTICLES_TO_IDS.get("minecraft:item_slime").unwrap(),
            Particle::ItemCobweb => *PARTICLES_TO_IDS.get("minecraft:item_cobweb").unwrap(),
            Particle::ItemSnowball => *PARTICLES_TO_IDS.get("minecraft:item_snowball").unwrap(),
            Particle::LargeSmoke => *PARTICLES_TO_IDS.get("minecraft:large_smoke").unwrap(),
            Particle::Lava => *PARTICLES_TO_IDS.get("minecraft:lava").unwrap(),
            Particle::Mycelium => *PARTICLES_TO_IDS.get("minecraft:mycelium").unwrap(),
            Particle::Note => *PARTICLES_TO_IDS.get("minecraft:note").unwrap(),
            Particle::Poof => *PARTICLES_TO_IDS.get("minecraft:poof").unwrap(),
            Particle::Portal => *PARTICLES_TO_IDS.get("minecraft:portal").unwrap(),
            Particle::Rain => *PARTICLES_TO_IDS.get("minecraft:rain").unwrap(),
            Particle::Smoke => *PARTICLES_TO_IDS.get("minecraft:smoke").unwrap(),
            Particle::WhiteSmoke => *PARTICLES_TO_IDS.get("minecraft:white_smoke").unwrap(),
            Particle::Sneeze => *PARTICLES_TO_IDS.get("minecraft:sneeze").unwrap(),
            Particle::Spit => *PARTICLES_TO_IDS.get("minecraft:spit").unwrap(),
            Particle::SquidInk => *PARTICLES_TO_IDS.get("minecraft:squid_ink").unwrap(),
            Particle::SweepAttack => *PARTICLES_TO_IDS.get("minecraft:sweep_attack").unwrap(),
            Particle::TotemOfUndying => {
                *PARTICLES_TO_IDS.get("minecraft:totem_of_undying").unwrap()
            }
            Particle::Underwater => *PARTICLES_TO_IDS.get("minecraft:underwater").unwrap(),
            Particle::Splash => *PARTICLES_TO_IDS.get("minecraft:splash").unwrap(),
            Particle::Witch => *PARTICLES_TO_IDS.get("minecraft:witch").unwrap(),
            Particle::BubblePop => *PARTICLES_TO_IDS.get("minecraft:bubble_pop").unwrap(),
            Particle::CurrentDown => *PARTICLES_TO_IDS.get("minecraft:current_down").unwrap(),
            Particle::BubbleColumnUp => {
                *PARTICLES_TO_IDS.get("minecraft:bubble_column_up").unwrap()
            }
            Particle::Nautilus => *PARTICLES_TO_IDS.get("minecraft:nautilus").unwrap(),
            Particle::Dolphin => *PARTICLES_TO_IDS.get("minecraft:dolphin").unwrap(),
            Particle::CampfireCosySmoke => {
                *PARTICLES_TO_IDS.get("minecraft:campfire_cosy").unwrap()
            }
            Particle::CampfireSignalSmoke => *PARTICLES_TO_IDS
                .get("minecraft:campfire_signal_smoke")
                .unwrap(),
            Particle::DrippingHoney => *PARTICLES_TO_IDS.get("minecraft:dripping_honey").unwrap(),
            Particle::FallingHoney => *PARTICLES_TO_IDS.get("minecraft:falling_honey").unwrap(),
            Particle::LandingHoney => *PARTICLES_TO_IDS.get("minecraft:landing_honey").unwrap(),
            Particle::FallingNectar => *PARTICLES_TO_IDS.get("minecraft:falling_nectar").unwrap(),
            Particle::FallingSporeBlossom => *PARTICLES_TO_IDS
                .get("minecraft:falling_spore_blossom")
                .unwrap(),
            Particle::Ash => *PARTICLES_TO_IDS.get("minecraft:ash").unwrap(),
            Particle::CrimsonSpore => *PARTICLES_TO_IDS.get("minecraft:crimson_spore").unwrap(),
            Particle::WarpedSpore => *PARTICLES_TO_IDS.get("minecraft:warped_spore").unwrap(),
            Particle::SporeBlossomAir => {
                *PARTICLES_TO_IDS.get("minecraft:spore_blossom_air").unwrap()
            }
            Particle::DrippingObsidianTear => *PARTICLES_TO_IDS
                .get("minecraft:dripping_obsidian_tear")
                .unwrap(),
            Particle::FallingObsidianTear => *PARTICLES_TO_IDS
                .get("minecraft:falling_obsidian_tear")
                .unwrap(),
            Particle::LandingObsidianTear => *PARTICLES_TO_IDS
                .get("minecraft:landing_obsidian_tear")
                .unwrap(),
            Particle::ReversePortal => *PARTICLES_TO_IDS.get("minecraft:reverse_portal").unwrap(),
            Particle::WhiteAsh => *PARTICLES_TO_IDS.get("minecraft:white_ash").unwrap(),
            Particle::SmallFlame => *PARTICLES_TO_IDS.get("minecraft:small_flame").unwrap(),
            Particle::Snowflake => *PARTICLES_TO_IDS.get("minecraft:snowflake").unwrap(),
            Particle::DrippingDripstoneLava => *PARTICLES_TO_IDS
                .get("minecraft:dripping_dripstone_lava")
                .unwrap(),
            Particle::FallingDripstoneLava => *PARTICLES_TO_IDS
                .get("minecraft:falling_dripstone_lava")
                .unwrap(),
            Particle::DrippingDripstoneWater => *PARTICLES_TO_IDS
                .get("minecraft:dripping_dripstone_water")
                .unwrap(),
            Particle::FallingDripstoneWater => *PARTICLES_TO_IDS
                .get("minecraft:falling_dripstone_water")
                .unwrap(),
            Particle::GlowSquidInk => *PARTICLES_TO_IDS.get("minecraft:glow_squid_ink").unwrap(),
            Particle::Glow => *PARTICLES_TO_IDS.get("minecraft:glow").unwrap(),
            Particle::WaxOn => *PARTICLES_TO_IDS.get("minecraft:wax_on").unwrap(),
            Particle::WaxOff => *PARTICLES_TO_IDS.get("minecraft:wax_off").unwrap(),
            Particle::ElectricSpark => *PARTICLES_TO_IDS.get("minecraft:electric_spark").unwrap(),
            Particle::Scrape => *PARTICLES_TO_IDS.get("minecraft:scrape").unwrap(),
            Particle::Shriek { .. } => *PARTICLES_TO_IDS.get("minecraft:shriek").unwrap(),
            Particle::EggCrack => *PARTICLES_TO_IDS.get("minecraft:egg_crack").unwrap(),
            Particle::DustPlume => *PARTICLES_TO_IDS.get("minecraft:dust_plume").unwrap(),
            Particle::TrialSpawnerDetection => *PARTICLES_TO_IDS
                .get("minecraft:trial_spawner_detection")
                .unwrap(),
            Particle::TrailSpawnerDetectionOminous => *PARTICLES_TO_IDS
                .get("minecraft:trail_spawner_detection_ominous")
                .unwrap(),
            Particle::VaultConnection => {
                *PARTICLES_TO_IDS.get("minecraft:vault_connection").unwrap()
            }
            Particle::DustPillar(..) => *PARTICLES_TO_IDS.get("minecraft:dust_pillar").unwrap(),
            Particle::OminousSpawning => {
                *PARTICLES_TO_IDS.get("minecraft:ominous_spawning").unwrap()
            }
            Particle::RaidOmen => *PARTICLES_TO_IDS.get("minecraft:raid_omen").unwrap(),
            Particle::TrailOmen => *PARTICLES_TO_IDS.get("minecraft:trail_omen").unwrap(),
            Particle::BlockCrumble(..) => *PARTICLES_TO_IDS.get("minecraft:block_crumble").unwrap(),
        }
    }
}

pub static PARTICLES_TO_IDS: LazyLock<IdTable<String>> = LazyLock::new(|| {
    let mut particles_to_ids = IdTable::new();
    DATA.registries
        .get("minecraft:particle_type")
        .unwrap()
        .entries
        .iter()
        .for_each(|(name, id)| {
            particles_to_ids.insert(name.clone(), *id);
        });
    particles_to_ids
});
