use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

use generated::{
    report::{
        blocks::GeneratedReportBlocks, packets::GeneratedReportPackets,
        registries::GeneratedReportRegistries, GeneratedReport,
    },
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
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Packages version \"{0}\" doesn't have download for \"{1}\"")]
    InvalidDownload(String, String),
    #[error("Version \"{0}\" not found in version manifest")]
    VersionNotFound(String),
    #[error("Invalid registry path")]
    InvalidRegistryPath,
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

pub fn generate_generated_code<P1: AsRef<Path>, P2: AsRef<Path>, P3: AsRef<Path>>(
    generated_directory: P1,
    generated_code_output: P2,
    generated_json_output: P3,
    skip_format: bool,
) -> Result<(), GeneratedError> {
    let registry = GeneratedRegistry::open(generated_directory);

    let mut generated_code: BTreeMap<String, String> = BTreeMap::new();
    let mut generated_report_json: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    fn generate_report<T: GeneratedReport>(
        registry: &GeneratedRegistry,
        generated_code: &mut BTreeMap<String, String>,
        generated_report_json: &mut BTreeMap<String, serde_json::Value>,
    ) -> Result<(), GeneratedError> {
        registry
            .report::<T>()?
            .code()?
            .into_iter()
            .for_each(|code| match code {
                generated::report::GeneratedReportCode::Code(module_name, module_code) => {
                    generated_code.insert(module_name, module_code);
                }
                generated::report::GeneratedReportCode::Json(key, value) => {
                    generated_report_json.insert(key, value);
                }
            });
        Ok(())
    }

    generate_report::<GeneratedReportPackets>(
        &registry,
        &mut generated_code,
        &mut generated_report_json,
    )?;
    generate_report::<GeneratedReportBlocks>(
        &registry,
        &mut generated_code,
        &mut generated_report_json,
    )?;
    generate_report::<GeneratedReportRegistries>(
        &registry,
        &mut generated_code,
        &mut generated_report_json,
    )?;

    let code = format!(
        "#![allow(warnings)]\n/// Code inside here (pkmc-defs/src/generated/generated.rs) & \"pkmc-defs/src/generated/generated.json\" generated by pkmc-generated, see pkmc-defs/README.md on how to generate this.\n{}",
        generated_code
            .iter()
            .sorted_by(|(module_name_1, _), (module_name_2, _)| module_name_1.cmp(module_name_2))
            .map(|(module_name, code)| format!("pub mod {} {{\n{}\n}}", module_name, code))
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    std::fs::write(&generated_code_output, code)?;

    if !skip_format {
        std::process::Command::new("cargo")
            .arg("fmt")
            .arg("--")
            .arg(generated_code_output.as_ref())
            .output()?;
    }

    let json = serde_json::to_string(&generated_report_json)?;
    let mut json_compressed = Vec::new();
    flate2::write::GzEncoder::new(&mut json_compressed, flate2::Compression::best())
        .write_all(json.as_ref())?;
    std::fs::write(&generated_json_output, &json_compressed)?;

    Ok(())
}
