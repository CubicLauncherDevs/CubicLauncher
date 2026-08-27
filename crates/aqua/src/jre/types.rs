use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

#[derive(Debug, Clone)]
pub struct JrePackage {
    pub major_version: u8,
    pub java_version: String,
    pub download_url: String,
    pub filename: String,
    pub distro_version: Vec<u32>,
    pub sha256_hash: Option<String>,
    pub vendor: &'static str,
    pub archive_format: ArchiveFormat,
    /// Known download size in bytes, when provided by the vendor API.
    pub size: Option<u64>,
}

impl JrePackage {
    pub fn format(&self) -> ArchiveFormat {
        self.archive_format
    }

    pub fn is_tar_gz(&self) -> bool {
        self.archive_format == ArchiveFormat::TarGz
    }

    pub fn is_zip(&self) -> bool {
        self.archive_format == ArchiveFormat::Zip
    }

    pub fn java_binary_name(&self) -> &'static str {
        if cfg!(target_os = "windows") {
            "javaw.exe"
        } else {
            "java"
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JreStatus {
    pub version: u8,
    pub installed: bool,
    pub java_version: Option<String>,
}
