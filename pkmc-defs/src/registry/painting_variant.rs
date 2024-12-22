use serde::{Deserialize, Serialize};

use super::FormattedText;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PaintingVariant {
    asset_id: String,
    height: i32,
    width: i32,
    title: FormattedText,
    author: FormattedText,
}
