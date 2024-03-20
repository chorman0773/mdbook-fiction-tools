use std::{collections::HashMap, path::PathBuf};

use serde_derive::Deserialize;

use crate::config::SerList;

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct EpubConfig {
    pub output: Option<SerList<EpubOutputType>>,
    pub always_include: Vec<PathBuf>,
    pub save_temps: bool,
    pub output_files: OutputFileSpec,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct OutputFileSpec {
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
