pub mod report;

use std::path::{Path, PathBuf};

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
}
