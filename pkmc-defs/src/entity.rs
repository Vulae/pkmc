use std::sync::LazyLock;

use pkmc_util::IdTable;

use crate::generated::DATA;

pub fn entity_type_id(name: &str) -> Option<i32> {
    ENTITIES_TO_IDS.get(name).cloned()
}

pub static ENTITIES_TO_IDS: LazyLock<IdTable<String>> = LazyLock::new(|| {
    let registry = DATA.registries.get("minecraft:entity_type").unwrap();
    let mut entities_to_ids = IdTable::new();
    registry.entries.iter().for_each(|(name, id)| {
        entities_to_ids.insert(name.to_owned(), *id);
    });
    entities_to_ids
});
