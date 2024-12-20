pub mod packets;

use std::path::Path;

use serde::de::DeserializeOwned;

use crate::GeneratedError;

pub trait GeneratedReport: DeserializeOwned {
    const INPUT_FILE: &'static str;
    fn load<P: AsRef<Path>>(report_directory: P) -> Result<Self, GeneratedError> {
        let mut report_file = report_directory.as_ref().to_path_buf();
        report_file.push(Self::INPUT_FILE);
        Ok(serde_json::from_str(&std::fs::read_to_string(
            report_file,
        )?)?)
    }
    fn code(&self) -> Result<String, GeneratedError>;
}
