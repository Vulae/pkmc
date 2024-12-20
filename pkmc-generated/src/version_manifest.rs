use serde::Deserialize;

use crate::{packages_version::PackagesVersion, GeneratedError};

pub const VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[derive(Deserialize)]
pub struct VersionManifestLatest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Deserialize)]
pub struct VersionManifestVersion {
    pub id: String,
    // TODO: Make this an enum.
    pub r#type: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

impl VersionManifestVersion {
    pub fn fetch(&self) -> Result<PackagesVersion, GeneratedError> {
        Ok(reqwest::blocking::get(&self.url)?.json::<PackagesVersion>()?)
    }
}

#[derive(Deserialize)]
pub struct VersionManifest {
    pub latest: VersionManifestLatest,
    pub versions: Vec<VersionManifestVersion>,
}

impl VersionManifest {
    pub fn fetch() -> Result<Self, GeneratedError> {
        Ok(reqwest::blocking::get(VERSION_MANIFEST_URL)?.json::<VersionManifest>()?)
    }

    pub fn get_version(&self, version_id: &str) -> Option<&VersionManifestVersion> {
        self.versions
            .iter()
            .find(|version| version.id == version_id)
    }
}
