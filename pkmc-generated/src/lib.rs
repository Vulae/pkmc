use std::{collections::HashMap, io::Read, path::Path};

use generated::{
    report::{packets::GeneratedReportPackets, GeneratedReport},
    GeneratedRegistry,
};
use itertools::Itertools;
use thiserror::Error;
use version_manifest::VersionManifest;

pub mod generated;
pub mod packages_version;
pub mod version_manifest;

#[derive(Error, Debug)]
pub enum GeneratedError {
    #[error("{0:?}")]
    IoError(#[from] std::io::Error),
    #[error("{0:?}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0:?}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Packages version \"{0}\" doesn't have download for \"{1}\"")]
    InvalidDownload(String, String),
    #[error("Version \"{0}\" not found in version manifest")]
    VersionNotFound(String),
}

pub fn download_server_jar<P: AsRef<Path>>(
    version_id: &str,
    output_file: P,
) -> Result<(), GeneratedError> {
    // TODO: Error handling for jar_file parent
    std::fs::create_dir_all(output_file.as_ref().parent().unwrap())?;

    let manifest = VersionManifest::fetch()?;
    let manifest_version = manifest
        .get_version(version_id)
        .ok_or(GeneratedError::VersionNotFound(version_id.to_owned()))?;
    let package_version = manifest_version.fetch()?;

    let download = package_version.download("server")?;

    // TODO: Stream the file instead.
    std::fs::write(&output_file, download.bytes()?)?;

    Ok(())
}

pub fn extract_generated_data<P1: AsRef<Path>, P2: AsRef<Path>>(
    jar_file: P1,
    output_directory: P2,
    stdout: bool,
) -> Result<(), GeneratedError> {
    std::fs::create_dir_all(&output_directory)?;

    let child = std::process::Command::new("java")
        // TODO: Error handling for jar_file with no parent
        .current_dir(jar_file.as_ref().parent().unwrap())
        .arg("-DbundlerMainClass=net.minecraft.data.Main")
        .arg("-jar")
        // TODO: Error handling for jar_file with no file name
        .arg(jar_file.as_ref().file_name().unwrap())
        .arg("--all")
        .arg("--output")
        .arg(output_directory.as_ref().canonicalize()?)
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    // TODO: Stream stdout
    let mut output = child.stdout.unwrap();
    let mut str = String::new();
    output.read_to_string(&mut str)?;
    if stdout {
        println!("{}", str);
    }

    Ok(())
}

pub fn generate_generated_code<P1: AsRef<Path>, P2: AsRef<Path>>(
    generated_directory: P1,
    generated_source_output: P2,
    skip_format: bool,
) -> Result<(), GeneratedError> {
    let registry = GeneratedRegistry::open(generated_directory);

    let mut sources: HashMap<String, String> = HashMap::new();

    sources.insert(
        "packet".to_owned(),
        registry.report::<GeneratedReportPackets>()?.code()?,
    );

    let code = format!(
        "#![allow(warnings)]\n/// Code generated by pkmc-generated, see pkmc-defs/README.md on how to generate this.\n{}",
        sources
            .iter()
            .sorted_by(|(module_name_1, _), (module_name_2, _)| module_name_1.cmp(module_name_2))
            .map(|(module_name, code)| format!("pub mod {} {{\n{}\n}}", module_name, code))
            .collect::<Vec<_>>()
            .join("\n")
    );
    std::fs::write(&generated_source_output, code)?;

    if !skip_format {
        std::process::Command::new("cargo")
            .arg("fmt")
            .arg("--")
            .arg(generated_source_output.as_ref())
            .output()?;
    }

    Ok(())
}