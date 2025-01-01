use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    path::PathBuf,
};

use serde_derive::Deserialize;
use uuid::Uuid;

use crate::config::{FileIds, SerList, SharedConfig};

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct EpubConfig {
    #[serde(flatten)]
    pub shared: SharedConfig,
    pub file_ids: FileIds<PackageId>,
}

impl Deref for EpubConfig {
    type Target = SharedConfig;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

#[derive(Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum PackageId {
    Uuid { uuid: Uuid },
    Oid { oid: String },
    Isbn { isbn: String },
}

impl core::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("urn:")?;
        match self {
            PackageId::Uuid { uuid } => f.write_fmt(format_args!("uuid:{uuid}")),
            PackageId::Oid { oid } => f.write_fmt(format_args!("oid:{oid}")),
            PackageId::Isbn { isbn } => f.write_fmt(format_args!("isbn:{isbn}")),
        }
    }
}
