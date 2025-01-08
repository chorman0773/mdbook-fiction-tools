use std::ops::Deref;

use serde::Deserialize;
use uuid::Uuid;

use crate::config::{FileIds, SharedConfig};

#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case", default)]
pub struct PdfConfig {
    #[serde(flatten)]
    pub base: SharedConfig,
    pub file_ids: FileIds<Uuid>,
}

impl Deref for PdfConfig {
    type Target = SharedConfig;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
