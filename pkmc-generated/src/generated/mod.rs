pub mod report;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use report::GeneratedReport;

use crate::GeneratedError;

#[derive(Debug)]
pub struct GeneratedRegistry {
    directory: PathBuf,
}

impl GeneratedRegistry {
    pub fn open<P: AsRef<Path>>(directory: P) -> Self {
        Self {
            directory: directory.as_ref().to_path_buf(),
        }
    }

    pub fn report<T>(&self) -> Result<T, GeneratedError>
    where
        T: GeneratedReport,
    {
        let mut report_directory = self.directory.clone();
        report_directory.push("reports");
        T::load(&report_directory)
    }

    pub fn data<P: AsRef<Path>>(&self, path: P) -> Result<serde_json::Value, GeneratedError> {
        let mut data_directory = self.directory.clone();
        data_directory.push("data");
        let mut data_file = data_directory.clone();
        data_file.push(&path);
        Ok(serde_json::Value::from_str(&std::fs::read_to_string(
            &data_file,
        )?)?)
    }

    pub fn enumerate_data(
        &self,
    ) -> Result<HashMap<String, HashMap<String, serde_json::Value>>, GeneratedError> {
        // I know this is pretty horrible.
        // Please don't look at this (˶˃⤙˂˶)

        fn visit_dirs<F>(dir: &Path, mut cb: F) -> std::io::Result<()>
        where
            F: FnMut(&Path, &[String]),
        {
            let mut stack: Vec<(PathBuf, Vec<String>)> = Vec::new();
            stack.push((dir.to_path_buf(), Vec::new()));

            while let Some((directory, sections)) = stack.pop() {
                for entry in std::fs::read_dir(directory)? {
                    let entry = entry?;
                    let path = entry.path();
                    let mut sections = sections.clone();
                    sections.push(path.file_name().unwrap().to_string_lossy().to_string());
                    if path.is_dir() {
                        stack.push((path, sections));
                    } else {
                        cb(&path, &sections);
                    }
                }
            }

            Ok(())
        }

        fn registry_path(path: &[String]) -> Result<(String, String), GeneratedError> {
            if path.len() < 3 {
                return Err(GeneratedError::InvalidRegistryPath);
            }
            Ok((
                format!(
                    "{}:{}",
                    path.first().ok_or(GeneratedError::InvalidRegistryPath)?,
                    path[1..path.len() - 1].join("/")
                ),
                path.last()
                    .ok_or(GeneratedError::InvalidRegistryPath)?
                    .to_owned(),
            ))
        }

        let mut data_directory = self.directory.clone();
        data_directory.push("data");

        let mut regestries = HashMap::new();

        let mut registry_files = Vec::new();
        visit_dirs(&data_directory, |file, path| {
            registry_files.push((file.to_path_buf(), path.to_vec().into_boxed_slice()));
        })?;

        registry_files.iter().try_for_each(|(file, path)| {
            let (name, item) = registry_path(path)?;
            let item = item.replace(".json", "");

            // Filter out some unneeded data.
            if name.starts_with("minecraft:datapacks/") || name.starts_with("minecraft:recipe") {
                return Ok(());
            }

            let json = serde_json::Value::from_str(&std::fs::read_to_string(file)?)?;

            let registry = regestries.entry(name).or_insert_with(HashMap::new);
            registry.insert(item, json);

            Ok::<_, GeneratedError>(())
        })?;

        Ok(regestries)
    }
}
