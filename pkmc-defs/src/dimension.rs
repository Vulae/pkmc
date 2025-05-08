use pkmc_util::IdTable;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Dimension {
    name: String,
}

impl Dimension {
    pub fn new<N: ToString>(name: N) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self, mapper: &IdTable<Dimension>) -> Option<i32> {
        mapper.get(self).cloned()
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Self {
            name: "minecraft:overworld".to_owned(),
        }
    }
}

impl From<String> for Dimension {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Dimension {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
