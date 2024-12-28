use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    path::PathBuf,
};

use serde_derive::Deserialize;
use uuid::Uuid;

use crate::config::{SerList, SharedConfig};

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct EpubConfig {
    #[serde(flatten)]
    pub shared: SharedConfig,
    pub file_ids: EpubFileIds,
}

impl Deref for EpubConfig {
    type Target = SharedConfig;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct EpubFileIds {
    pub full: Option<EpubPackageId>,
    #[serde(flatten)]
    pub individual_files: HashMap<String, EpubPackageId>,
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
