use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use serde_derive::Deserialize;
use uuid::Uuid;

use crate::config::SerList;

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct EpubConfig {
    pub output: Option<SerList<EpubOutputType>>,
    pub always_include: HashSet<PathBuf>,
    pub save_temps: bool,
    pub output_files: OutputFileSpec,
    pub file_ids: EpubFileIds,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct EpubFileIds {
    pub full: Option<EpubPackageId>,
    #[serde(flatten)]
    pub individual_files: HashMap<String, EpubPackageId>,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct OutputFileSpec {
    #[serde(default)]
    pub full: Option<PathBuf>,
    #[serde(flatten)]
    pub individual_files: HashMap<String, PathBuf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpubOutputType {
    Chapter,
    Part,
    Full,
}

#[derive(Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum EpubPackageId {
    Uuid { uuid: Uuid },
    Oid { oid: String },
    Isbn { isbn: String },
}

impl core::fmt::Display for EpubPackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("urn:")?;
        match self {
            EpubPackageId::Uuid { uuid } => f.write_fmt(format_args!("uuid:{uuid}")),
            EpubPackageId::Oid { oid } => f.write_fmt(format_args!("oid:{oid}")),
            EpubPackageId::Isbn { isbn } => f.write_fmt(format_args!("isbn:{isbn}")),
        }
    }
}
