use std::{borrow::Cow, path::PathBuf};

use xml::{namespace::NS_NO_PREFIX, writer::XmlEvent, EventWriter};

use super::{
    config::PackageId,
    info::{EpubFileInfo, EPUB_UNIQUE_IDENTIFIER_ID},
};

pub use super::info::{NS_DC_PREFIX, NS_DC_URI};

pub const NS_OPF_URI: &str = "http://www.idpf.org/2007/opf";

pub const EPUB_PACKAGE_MEDIA_TYPE: &str = "application/oebps-package+xml";

pub const EPUB_VERSION: &str = "3.0";

pub const OCF_CONTAINER_VERSION: &str = "1.0";

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum ItemProperty {
    CoverImage,
    MathML,
    Nav,
    RemoteResources,
    Scripted,
    Svg,
}

impl core::fmt::Display for ItemProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemProperty::CoverImage => f.write_str("cover-image"),
            ItemProperty::MathML => f.write_str("mathml"),
            ItemProperty::Nav => f.write_str("nav"),
            ItemProperty::RemoteResources => f.write_str("remote-resources"),
            ItemProperty::Scripted => f.write_str("scripted"),
            ItemProperty::Svg => f.write_str("svg"),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ManifestItem {
    pub id: String,
    pub path: PathBuf,
    pub media_type: Cow<'static, str>,
    pub properties: Vec<ItemProperty>,
    pub fallback: Option<String>,
    pub spine: bool,
}

impl ManifestItem {
    pub fn serialize_item<W: std::io::Write>(
        &self,
        writer: &mut EventWriter<W>,
    ) -> xml::writer::Result<()> {
        use core::fmt::Write as _;
        let mut properties_value = String::new();
        let mut start_event = XmlEvent::start_element("item")
            .attr("href", self.path.as_os_str().to_str().unwrap())
            .attr("id", &self.id)
            .attr("media-type", &self.media_type);

        if !self.properties.is_empty() {
            let mut sep = "";

            for prop in &self.properties {
                properties_value.push_str(sep);
                sep = " ";
                write!(properties_value, "{}", prop).unwrap();
            }

            start_event = start_event.attr("properties", &properties_value);
        }

        if let Some(fallback) = &self.fallback {
            start_event = start_event.attr("fallback", fallback);
        }

        writer.write(start_event)?;

        writer.write(XmlEvent::end_element())
    }

    pub fn serialize_spine<W: std::io::Write>(
        &self,
        writer: &mut EventWriter<W>,
    ) -> xml::writer::Result<()> {
        if self.spine {
            writer.write(XmlEvent::start_element("itemref").attr("idref", &self.id))?;
            writer.write(XmlEvent::end_element())
        } else {
            Ok(())
        }
    }
}

pub struct EpubPackage {
    pub info: EpubFileInfo,
    pub manifest: Vec<ManifestItem>,
}

impl EpubPackage {
    pub fn serialize<W: std::io::Write>(
        &self,
        writer: &mut EventWriter<W>,
    ) -> xml::writer::Result<()> {
        writer.write(
            XmlEvent::start_element("package")
                .ns(NS_NO_PREFIX, NS_OPF_URI)
                .ns(NS_DC_PREFIX, NS_DC_URI)
                .attr("unique-identifier", EPUB_UNIQUE_IDENTIFIER_ID)
                .attr("version", EPUB_VERSION),
        )?;
        writer.write(XmlEvent::start_element("metadata"))?;
        self.info.write_metadata(writer)?;
        writer.write(XmlEvent::end_element())?; // </metadata>
        writer.write(XmlEvent::start_element("manifest"))?;
        for item in &self.manifest {
            item.serialize_item(writer)?;
        }
        writer.write(XmlEvent::end_element())?; // </manifest>

        writer.write(XmlEvent::start_element("spine"))?;
        for item in &self.manifest {
            item.serialize_spine(writer)?;
        }
        writer.write(XmlEvent::end_element())?; // </spine>
        writer.write(XmlEvent::end_element()) // </package>
    }
}
