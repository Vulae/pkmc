use serde::{Deserialize, Serialize};

use super::FormattedText;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TrimPattern {
    pub asset_id: String,
    pub template_item: String,
    pub description: FormattedText,
    pub decal: bool,
}
