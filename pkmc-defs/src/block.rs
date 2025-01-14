use std::{collections::BTreeMap, sync::LazyLock};

use pkmc_util::IdTable;
use serde::{Deserialize, Serialize};

use crate::generated::{generated, DATA};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[serde(transparent)]
pub struct BlockProperties(BTreeMap<String, String>);

impl BlockProperties {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn get<K: ToString>(&self, key: K) -> Option<&str> {
        self.0.get(&key.to_string()).map(|x| x.as_str())
    }

    pub fn contains<K: ToString>(&self, key: K) -> bool {
        self.0.contains_key(&key.to_string())
    }

    pub fn insert<K: ToString, V: ToString>(&mut self, key: K, value: V) -> Option<String> {
        self.0.insert(key.to_string(), value.to_string())
    }

    pub fn remove<K: ToString>(&mut self, key: K) -> Option<String> {
        self.0.remove(&key.to_string())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl<K: ToString, V: ToString, I: IntoIterator<Item = (K, V)>> From<I> for BlockProperties {
    fn from(value: I) -> Self {
        BlockProperties(
            value
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    #[serde(alias = "Name")]
    pub name: String,
    #[serde(alias = "Properties", default)]
    pub properties: BlockProperties,
}

impl Block {
    pub fn new_p<N: ToString, P: Into<BlockProperties>>(name: N, properties: P) -> Self {
        Self {
            name: name.to_string(),
            properties: properties.into(),
        }
    }

    pub fn new<N: ToString>(name: N) -> Self {
        Self::new_p(name, None::<(&str, &str)>)
    }

    pub fn without_properties(&self) -> Self {
        Self::new(&self.name)
    }

    pub fn air() -> Self {
        Self::new("minecraft:air")
    }

    pub fn is_air(&self) -> bool {
        //matches!(
        //    self.name.as_ref(),
        //    "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        //)
        self.id().map(generated::block::is_air).unwrap_or(false)
    }

    pub fn id(&self) -> Option<i32> {
        BLOCKS_TO_IDS.get(self).copied()
    }

    pub fn id_with_default_fallback(&self) -> Option<i32> {
        self.id().or_else(|| self.without_properties().id())
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::air()
    }
}

pub static BLOCKS_TO_IDS: LazyLock<IdTable<Block>> = LazyLock::new(|| {
    let mut blocks_to_ids = IdTable::new();
    DATA.block.iter().for_each(|(name, block)| {
        block.states.iter().for_each(|state| {
            if state.default {
                blocks_to_ids.insert(Block::new(name), state.id);
            }
            blocks_to_ids.insert(Block::new_p(name, state.properties.iter()), state.id);
        });
    });
    blocks_to_ids
});

#[cfg(test)]
mod test {
    use crate::block::{Block, BLOCKS_TO_IDS};

    #[test]
    fn test_blocks_to_ids() {
        assert_eq!(BLOCKS_TO_IDS.get(&Block::air()).copied(), Some(0));
        assert_eq!(
            BLOCKS_TO_IDS.get(&Block::new("minecraft:stone")).copied(),
            Some(1)
        );
        assert_eq!(
            BLOCKS_TO_IDS
                .get(&Block::new_p(
                    "minecraft:mushroom_stem",
                    [
                        ("down", false),
                        ("east", false),
                        ("north", false),
                        ("south", true),
                        ("up", false),
                        ("west", false),
                    ]
                ))
                .copied(),
            Some(6969)
        );
    }
}
