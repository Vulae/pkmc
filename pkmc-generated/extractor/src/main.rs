use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    io::Read as _,
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::{Deserialize, Serialize};

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

fn download<P: AsRef<Path>>(release: &str, output: P) -> Result<PathBuf, Box<dyn Error>> {
    let output = output.as_ref().to_path_buf();

    std::fs::create_dir_all(&output)?;

    // Download server.jar
    let manifest = VersionManifest::fetch()?;
    let manifest_version = manifest
        .get_version(release)
        // .ok_or(Err("Could not find version"))?;
        .unwrap();
    let package_version = manifest_version.fetch()?;

    let download = package_version.download("server")?;

    let mut output_file = output.clone();
    output_file.push("server.jar");

    std::fs::write(&output_file, download.bytes()?)?;

    let output_directory = output.canonicalize()?;

    // Extract definitions from server.jar
    let child = std::process::Command::new("java")
        .current_dir(&output)
        .arg("-DbundlerMainClass=net.minecraft.data.Main")
        .arg("-jar")
        .arg(output_file.file_name().unwrap())
        .arg("--all")
        .arg("--output")
        .arg(&output_directory)
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    // TODO: Stream stdout
    let mut output = child.stdout.unwrap();
    let mut str = String::new();
    output.read_to_string(&mut str)?;
    println!("{}", str);

    // NOTE: The original server.jar seems to just delete itself when done?????

    Ok(output_directory)
}

type Registry = BTreeMap<String, serde_json::Value>;
type Registries = BTreeMap<String, Registry>;

fn generate_registry_json<P: AsRef<Path>, M>(
    registry_path: P,
    namespace: &str,
    mapper: M,
) -> Result<Registry, Box<dyn Error>>
where
    M: Fn(serde_json::Value) -> Result<serde_json::Value, Box<dyn Error>>,
{
    let mut registry = Registry::new();

    registry_path.as_ref().read_dir()?.try_for_each(|entry| {
        let entry = entry?;
        if !entry.metadata()?.is_file() {
            eprintln!("WARNING: Found directory while evaluating registry entries.");
            return Ok(());
        }

        let entry_path = entry.path();
        let entry_name = entry_path
            .components()
            .last()
            .unwrap()
            .as_os_str()
            .to_string_lossy()
            .into_owned()
            .trim_end_matches(".json")
            .to_owned();

        let entry_data: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(entry_path)?)?;

        registry.insert(format!("{}:{}", namespace, entry_name), mapper(entry_data)?);

        Ok::<_, Box<dyn Error>>(())
    })?;

    Ok(registry)
}

fn enumerate_registries<P: AsRef<Path>>(
    registries_directory: P,
    root: bool,
) -> Result<Vec<(PathBuf, String)>, Box<dyn Error>> {
    let registries_directory = registries_directory.as_ref().to_path_buf();
    let top_name = registries_directory
        .components()
        .last()
        .unwrap()
        .as_os_str()
        .to_string_lossy()
        .into_owned();

    let mut directory_count = 0;
    let mut file_count = 0;

    let children = registries_directory
        .read_dir()?
        .map(|entry| {
            let entry = entry?;
            match entry.metadata()? {
                metadata if metadata.is_dir() => directory_count += 1,
                metadata if metadata.is_file() => file_count += 1,
                _ => unreachable!(),
            }
            Ok(entry)
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;

    match (directory_count, file_count) {
        (0, 0) => {
            eprintln!("WARNING: Empty registry");
            Ok(Vec::new())
        }
        (0, _) => Ok(vec![(registries_directory, top_name)]),
        (_, 0) => Ok(children
            .into_iter()
            .map(|child| {
                let children_registries = enumerate_registries(child.path(), false)?;
                Ok(children_registries
                    .into_iter()
                    .map(|(p1, p2)| {
                        (
                            p1,
                            if !root {
                                format!("{}/{}", top_name, p2)
                            } else {
                                p2
                            },
                        )
                    })
                    .collect::<Vec<_>>())
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?
            .into_iter()
            .flatten()
            .collect()),
        (_, _) => {
            eprintln!("WARNING: Skipped registry that has items & sub registries");
            Ok(Vec::new())
        }
    }
}

fn generate_registries_json<P: AsRef<Path>>(
    registry_directory: P,
) -> Result<Registries, Box<dyn Error>> {
    // TODO: Merge namespace data (Useless unless modded.)

    fn registry_filter(name: &str) -> bool {
        match name {
            name if name.starts_with("minecraft:advancement") => false,
            name if name.starts_with("minecraft:datapacks") => false,
            name if name.starts_with("minecraft:enchantment_provider") => false,
            name if name.starts_with("minecraft:loot_table") => false,
            "minecraft:recipe" => false, // TODO: Don't ignore this.
            name if name.starts_with("minecraft:tags") => false, // TODO: Don't ignore this.
            name if name.starts_with("minecraft:trial_spawner") => false, // ???
            "minecraft:worldgen/biome" => true,
            name if name.starts_with("minecraft:worldgen") => false, // ???
            "minecraft:enchantment" => false,
            "minecraft:test_environment" => false,
            "minecraft:test_instance" => false,
            "minecraft:test_spawner" => false,
            _ => true,
        }
    }

    let mut registries: Registries = Registries::new();

    registry_directory
        .as_ref()
        .read_dir()?
        .try_for_each(|entry| {
            let entry = entry?;
            if !entry.metadata()?.is_dir() {
                eprintln!("WARNING: Found file while evaluating namespace registries.");
                return Ok(());
            }

            let namespace_path = entry.path();
            let namespace_name = namespace_path
                .components()
                .last()
                .unwrap()
                .as_os_str()
                .to_string_lossy()
                .into_owned();

            enumerate_registries(namespace_path, true)?
                .into_iter()
                .try_for_each(|(registry_path, registry_name)| {
                    let registry_key = format!("{}:{}", namespace_name, registry_name);

                    if !registry_filter(&registry_key) {
                        return Ok(());
                    }

                    let registry = match registry_key.as_ref() {
                        "minecraft:worldgen/biome" => {
                            // This intentionally has less properties to reduce unneeded properties.
                            #[derive(Deserialize, Serialize)]
                            struct WorldgenBiome {
                                downfall: f32,
                                temperature: f32,
                                has_precipitation: bool,
                                effects: serde_json::Value,
                            }

                            generate_registry_json(registry_path, &namespace_name, |v| {
                                let parsed: WorldgenBiome = serde_json::from_value(v)?;
                                Ok(serde_json::to_value(parsed)?)
                            })?
                        }
                        _ => generate_registry_json(registry_path, &namespace_name, Ok)?,
                    };

                    registries.insert(registry_key, registry);

                    Ok::<_, Box<dyn Error>>(())
                })?;

            Ok::<_, Box<dyn Error>>(())
        })?;

    Ok(registries)
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    /// Minecraft version ID to download (e.g. "1.21.5")
    release: String,
    #[arg(short, long)]
    /// Output directory path (e.g. "extracted/")
    output: PathBuf,
    #[arg(short, long)]
    /// Skip download & extract (For testing purposes)
    skip_download: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args {
        release,
        output,
        skip_download,
    } = Args::parse();

    let output_directory = if !skip_download {
        download(&release, &output)?
    } else {
        output.canonicalize()?
    };

    let mut registry_directory = output_directory.clone();
    registry_directory.push("data");
    let registries = generate_registries_json(&registry_directory)?;

    let mut registries_file_path = output_directory.clone();
    registries_file_path.push("pkmc_merged_registries.json");
    std::fs::write(
        registries_file_path,
        serde_json::to_string_pretty(&registries)?,
    )?;

    Ok(())
}
