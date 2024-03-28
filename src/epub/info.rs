use time::format_description::well_known::Rfc3339;
use xml::{name::Name, writer::XmlEvent, EventWriter};

use super::config::EpubPackageId;

pub const NS_CONTAINER_URI: &str = "urn:oasis:names:tc:opendocument:xmlns:container";

pub const NS_DC_PREFIX: &str = "dc";
pub const NS_DC_URI: &str = "http://purl.org/dc/elements/1.1/";

pub const EPUB_UNIQUE_IDENTIFIER_ID: &str = "primary-ident";

pub struct EpubFileInfo {
    pub title: String,
    pub ident: EpubPackageId,
    pub lang: String,
    pub creators: Vec<String>,
}

impl EpubFileInfo {
    pub fn write_metadata<W: std::io::Write>(
        &self,
        w: &mut EventWriter<W>,
    ) -> xml::writer::Result<()> {
        let modified = time::OffsetDateTime::now_utc()
            .replace_nanosecond(0)
            .unwrap();
        w.write(
            XmlEvent::start_element(Name::prefixed("identifier", NS_DC_PREFIX))
                .attr("id", EPUB_UNIQUE_IDENTIFIER_ID),
        )?;
        w.write(XmlEvent::characters(&self.ident.to_string()))?;
        w.write(XmlEvent::end_element())?; // </dc:identifier>
        w.write(XmlEvent::start_element(Name::prefixed(
            "title",
            NS_DC_PREFIX,
        )))?;
        w.write(XmlEvent::characters(&self.title))?;
        w.write(XmlEvent::end_element())?; // </dc:title>
        w.write(XmlEvent::start_element(Name::prefixed(
            "language",
            NS_DC_PREFIX,
        )))?;
        w.write(XmlEvent::characters(&self.lang))?;
        w.write(XmlEvent::end_element())?; // </dc:language>
        for creator in &self.creators {
            w.write(XmlEvent::start_element(Name::prefixed(
                "creator",
                NS_DC_PREFIX,
            )))?;
            w.write(XmlEvent::characters(&creator))?;
            w.write(XmlEvent::end_element())? // </dc:creator>
        }

        w.write(XmlEvent::start_element("meta").attr("property", "dcterms:modified"))?;
        let text = modified.format(&Rfc3339).unwrap();
        w.write(XmlEvent::characters(&text))?;
        w.write(XmlEvent::end_element()) // </meta>
    }
}
