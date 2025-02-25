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

fn config_default_brand() -> String {
    "Vulae/pkmc".to_owned()
}

fn config_default_view_distance() -> u8 {
    12
}

fn config_default_entity_distance() -> f64 {
    256.0
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub address: String,
    #[serde(default = "config_default_brand")]
    pub brand: String,
    #[serde(default, rename = "compression-threshold")]
    pub compression_threshold: usize,
    #[serde(default, rename = "compression-level")]
    pub compression_level: u32,
    pub world: PathBuf,
    #[serde(default = "config_default_view_distance", rename = "view-distance")]
    pub view_distance: u8,
    #[serde(default = "config_default_entity_distance", rename = "entity-distance")]
    pub entity_distance: f64,
    #[serde(rename = "motd-text")]
    pub motd_text: Option<String>,
    #[serde(rename = "motd-icon")]
    pub motd_icon: Option<PathBuf>,
    #[serde(default, rename = "motd-icon-filtering-method")]
    pub motd_icon_filtering_method: ConfigImageFilteringMethod,
}

impl Config {
    /// Convert relative paths to absolute
    fn fix_paths(&mut self, config_file_path: PathBuf) -> Result<(), std::io::Error> {
        let config_directory_path = config_file_path
            .canonicalize()?
            .parent()
            .ok_or(std::io::ErrorKind::NotFound)?
            .to_path_buf();
        if self.world.is_relative() {
            let mut path = config_directory_path.clone();
            path.push(self.world.clone());
            self.world = path;
        }
        if let Some(ref mut icon) = self.motd_icon {
            if icon.is_relative() {
                let mut path = config_directory_path.clone();
                path.push(icon.clone());
                *icon = path;
            }
        }
        Ok(())
    }

    /// First file that is found is loaded as config.
    pub fn load<P: AsRef<Path>>(paths: &[P]) -> Result<Config, Box<dyn Error>> {
        for path in paths {
            match std::fs::read_to_string(path) {
                Ok(str) => {
                    return Ok({
                        let mut config: Config = toml::from_str(&str)?;
                        config.fix_paths(PathBuf::from(path.as_ref()))?;
                        config
                    })
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(Box::new(err)),
            }
        }
        Err("Could not find config file.".into())
    }
}
