use std::collections::HashMap;

use serde::Deserialize;

use crate::GeneratedError;

#[derive(Deserialize)]
pub struct PackagesVersionDownload {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Deserialize)]
pub struct PackagesVersion {
    pub arguments: serde_json::Value,
    #[serde(rename = "assetIndex")]
    pub asset_index: serde_json::Value,
    pub assets: String,
    #[serde(rename = "complianceLevel")]
    pub compliance_level: i64,
    pub downloads: HashMap<String, PackagesVersionDownload>,
    pub id: String,
    #[serde(rename = "javaVersion")]
    pub java_version: serde_json::Value,
    pub libraries: serde_json::Value,
    pub logging: serde_json::Value,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: i64,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub time: String,
    // TODO: Make this an enum.
    pub r#type: String,
}

impl PackagesVersion {
    pub fn download_url(&self, download: &str) -> Option<&str> {
        self.downloads
            .get(download)
            .map(|download| download.url.as_str())
    }

    pub fn download(&self, download: &str) -> Result<reqwest::blocking::Response, GeneratedError> {
        self.download_url(download)
            .ok_or(GeneratedError::InvalidDownload(
                self.id.to_owned(),
                download.to_owned(),
            ))
            .map(|url| Ok(reqwest::blocking::get(url)?))?
    }
}
