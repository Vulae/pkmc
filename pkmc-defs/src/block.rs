use std::{collections::BTreeMap, sync::LazyLock};

use pkmc_generated::{
    block::{Block, BLOCKS_REPORT},
    registry::BlockEntityType,
};
use pkmc_util::{nbt::NBT, IdTable};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[serde(transparent)]
pub struct DynamicBlockProperties(BTreeMap<String, String>);

impl DynamicBlockProperties {
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

impl<K: ToString, V: ToString, I: IntoIterator<Item = (K, V)>> From<I> for DynamicBlockProperties {
    fn from(value: I) -> Self {
        DynamicBlockProperties(
            value
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DynamicBlock {
    #[serde(alias = "Name")]
    pub name: String,
    #[serde(alias = "Properties", default)]
    pub properties: DynamicBlockProperties,
}

impl DynamicBlock {
    pub fn new_p<N: ToString, P: Into<DynamicBlockProperties>>(name: N, properties: P) -> Self {
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

    pub fn to_block(&self) -> Option<Block> {
        let id = *BLOCKS_TO_IDS
            .get(self)
            .or_else(|| BLOCKS_TO_IDS.get(&self.without_properties()))?;
        Block::from_id(id)
    }
}

impl Default for DynamicBlock {
    fn default() -> Self {
        Self::new("minecraft:air")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicBlockEntity {
    pub block: DynamicBlock,
    pub r#type: BlockEntityType,
    pub data: NBT,
}

impl DynamicBlockEntity {
    pub fn new(block: DynamicBlock, r#type: BlockEntityType, data: NBT) -> Self {
        Self {
            block,
            r#type,
            data,
        }
    }

    pub fn into_block(self) -> DynamicBlock {
        self.block
    }
}

pub static BLOCKS_TO_IDS: LazyLock<IdTable<DynamicBlock>> = LazyLock::new(|| {
    let mut blocks_to_ids = IdTable::new();
    BLOCKS_REPORT.0.iter().for_each(|(name, block)| {
        block.states.iter().for_each(|state| {
            if state.default {
                blocks_to_ids.insert(DynamicBlock::new(name), state.id);
            }
            blocks_to_ids.insert(DynamicBlock::new_p(name, state.properties.iter()), state.id);
        });
    });
    blocks_to_ids
});

#[cfg(test)]
mod test {
    use crate::block::{DynamicBlock, BLOCKS_TO_IDS};

    #[test]
    fn test_blocks_to_ids() {
        assert_eq!(
            BLOCKS_TO_IDS
                .get(&DynamicBlock::new("minecraft:air"))
                .copied(),
            Some(0)
        );
        assert_eq!(
            BLOCKS_TO_IDS
                .get(&DynamicBlock::new("minecraft:stone"))
                .copied(),
            Some(1)
        );
    }
}
