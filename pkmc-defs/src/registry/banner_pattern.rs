use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BannerPattern {
    pub asset_id: String,
    pub translation_key: String,
}
