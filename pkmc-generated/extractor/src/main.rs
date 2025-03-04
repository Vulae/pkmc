use std::{collections::HashMap, error::Error, io::Read as _, path::PathBuf};

use clap::Parser;
use serde::Deserialize;

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

    pub fn download(&self, download: &str) -> Result<reqwest::blocking::Response, Box<dyn Error>> {
        Ok(reqwest::blocking::get(
            self.download_url(download)
                // .ok_or(Err("Invalid download".into()))?,
                .unwrap(),
        )?)
    }
}

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
    pub fn fetch(&self) -> Result<PackagesVersion, Box<dyn Error>> {
        Ok(reqwest::blocking::get(&self.url)?.json::<PackagesVersion>()?)
    }
}

#[derive(Deserialize)]
pub struct VersionManifest {
    pub latest: VersionManifestLatest,
    pub versions: Vec<VersionManifestVersion>,
}

impl VersionManifest {
    pub fn fetch() -> Result<Self, Box<dyn Error>> {
        Ok(reqwest::blocking::get(VERSION_MANIFEST_URL)?.json::<VersionManifest>()?)
    }

    pub fn get_version(&self, version_id: &str) -> Option<&VersionManifestVersion> {
        self.versions
            .iter()
            .find(|version| version.id == version_id)
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    /// Minecraft version ID to download (e.g. "1.21.4")
    release: String,
    #[arg(short, long)]
    /// Output directory path (e.g. "extracted/")
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args { release, output } = Args::parse();

    std::fs::create_dir_all(&output)?;

    // Download server.jar
    let manifest = VersionManifest::fetch()?;
    let manifest_version = manifest
        .get_version(&release)
        // .ok_or(Err("Could not find version"))?;
        .unwrap();
    let package_version = manifest_version.fetch()?;

    let download = package_version.download("server")?;

    let mut output_file = output.clone();
    output_file.push("server.jar");

    std::fs::write(&output_file, download.bytes()?)?;

    // Extract definitions from server.jar
    let child = std::process::Command::new("java")
        .current_dir(&output)
        .arg("-DbundlerMainClass=net.minecraft.data.Main")
        .arg("-jar")
        .arg(output_file.file_name().unwrap())
        .arg("--all")
        .arg("--output")
        .arg(output.canonicalize()?)
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    // TODO: Stream stdout
    let mut output = child.stdout.unwrap();
    let mut str = String::new();
    output.read_to_string(&mut str)?;
    println!("{}", str);

    // NOTE: The original server.jar seems to just delete itself when done?????

    Ok(())
}
