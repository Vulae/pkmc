use pkmc_util::IdTable;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Biome {
    name: String,
}

impl Biome {
    pub fn new<N: ToString>(name: N) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn id(&self, mapper: &IdTable<Biome>) -> Option<i32> {
        mapper.get(self).cloned()
    }
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            name: "minecraft:the_void".to_owned(),
        }
    }
}

impl From<String> for Biome {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Biome {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
