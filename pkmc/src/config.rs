use std::{
    error::Error,
    path::{Path, PathBuf},
};

use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub enum ConfigImageFilteringMethod {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    #[default]
    Lanczos3,
}

impl ConfigImageFilteringMethod {
    pub fn to_image_rs_filtering_method(&self) -> image::imageops::FilterType {
        match self {
            ConfigImageFilteringMethod::Nearest => image::imageops::Nearest,
            ConfigImageFilteringMethod::Triangle => image::imageops::Triangle,
            ConfigImageFilteringMethod::CatmullRom => image::imageops::CatmullRom,
            ConfigImageFilteringMethod::Gaussian => image::imageops::Gaussian,
            ConfigImageFilteringMethod::Lanczos3 => image::imageops::Lanczos3,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct ConfigServerList {
    pub text: Option<String>,
    pub icon: Option<PathBuf>,
    #[serde(default, rename = "icon-filtering-method")]
    pub icon_filtering_method: ConfigImageFilteringMethod,
}

fn config_default_brand() -> String {
    "Vulae/pkmc".to_owned()
}

fn config_default_view_distance() -> u8 {
    12
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub address: String,
    #[serde(default = "config_default_brand")]
    pub brand: String,
    #[serde(default, rename = "server-list")]
    pub server_list: ConfigServerList,
    #[serde(default, rename = "compression-threshold")]
    pub compression_threshold: usize,
    #[serde(default, rename = "compression-level")]
    pub compression_level: u32,
    pub world: PathBuf,
    #[serde(default = "config_default_view_distance", rename = "view-distance")]
    pub view_distance: u8,
}

impl Config {
    /// First file that is found is loaded as config.
    pub fn load<P: AsRef<Path>>(paths: &[P]) -> Result<Config, Box<dyn Error>> {
        for path in paths {
            match std::fs::read_to_string(path) {
                Ok(str) => return Ok(toml::from_str(&str)?),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(Box::new(err)),
            }
        }
        Err("Could not find config file.".into())
    }
}
