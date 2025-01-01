use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use xml::{
    name::Name,
    namespace::{Namespace, NS_NO_PREFIX, NS_XML_PREFIX, NS_XML_URI},
    writer::XmlEvent,
    EmitterConfig, EventWriter,
};
use zip::write::{FileOptions, ZipWriter};

use crate::{
    bookir::{
        nav::{NavHeading, NavTree},
        Book,
    },
    epub::{
        info::NS_CONTAINER_URI,
        package::{ItemProperty, ManifestItem, EPUB_PACKAGE_MEDIA_TYPE},
    },
    helpers::{media_type_from_file, name_to_id, visit_chapters},
};

use info::EpubFileInfo;

pub mod config;
pub mod info;
pub mod package;
#[cfg(feature = "epub-signatures")]
pub mod signature;
pub mod style;
pub mod xhtml;

pub const NS_EPUB_PREFIX: &str = "epub";
pub const NS_EPUB_URI: &str = "http://www.idpf.org/2007/ops";

pub fn write_nav<W: std::io::Write>(
    tree: &NavTree,
    w: &mut EventWriter<W>,
) -> xml::writer::Result<()> {
    w.write(XmlEvent::start_element("ol"))?;

    for node in tree {
        match &node.heading {
            NavHeading::Chapter(title, chapter) => {
                let mut path = chapter.dest_path.to_path_buf();
                path.set_extension("xhtml");

                w.write(XmlEvent::start_element("li"))?;

                w.write(XmlEvent::start_element("a").attr("href", &path.to_string_lossy()))?;
                w.write(XmlEvent::characters(title))?;
                w.write(XmlEvent::end_element())?;
            }
            NavHeading::Heading(head) => {
                w.write(XmlEvent::start_element("li"))?;
                w.write(XmlEvent::start_element("span"))?;
                w.write(XmlEvent::characters(head))?;
                w.write(XmlEvent::end_element())?;
            }
            NavHeading::UnboundChapter(title) => {
                w.write(XmlEvent::start_element("li"))?;
                w.write(XmlEvent::start_element("span"))?;
                w.write(XmlEvent::characters(title))?;
                w.write(XmlEvent::end_element())?;
            }
        }
        if let Some(children) = &node.children {
            write_nav(children, w)?;
        }
        w.write(XmlEvent::end_element())?;
    }
    w.write(XmlEvent::end_element())
}

pub fn write_epub<W: std::io::Write + std::io::Seek>(
    writer: W,
    book: Book,
    info: EpubFileInfo,
    package_id: String,
) -> std::io::Result<()> {
    use std::io::Write;
    let zip_file_options = FileOptions::default();
    let xml_config = EmitterConfig::new();
    let mut zip = ZipWriter::new(writer);

    zip.set_comment(&info.title);

    zip.start_file(
        "mimetype",
        FileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    write!(zip, "application/epub+zip")?;

    let mut manifest = Vec::new();

    for item in book.tree.nested() {
        match &item.heading {
            crate::bookir::nav::NavHeading::Chapter(title, chapter) => {
                let in_file_path = {
                    let mut path = chapter.dest_path.to_path_buf();
                    path.set_extension("xhtml");
                    path
                };

                let str = in_file_path.to_string_lossy();

                zip.start_file(str, zip_file_options.clone())?;

                let mut writer = EventWriter::new_with_config(&mut zip, xml_config.clone());
                xhtml::write_chapter(chapter, &mut writer).map_err(xhtml::xml_to_io_error)?;

                let spine_item = ManifestItem {
                    id: name_to_id(title),
                    path: in_file_path,
                    media_type: Cow::Borrowed(xhtml::XHTML_MEDIA),
                    properties: vec![],
                    fallback: None,
                    spine: true,
                };

                manifest.push(spine_item);
            }
            _ => {}
        }
    }

    let nav_item = ManifestItem {
        id: format!("nav-toc"),
        path: PathBuf::from("nav.xhtml"),
        media_type: Cow::Borrowed(xhtml::XHTML_MEDIA),
        properties: vec![ItemProperty::Nav],
        fallback: None,
        spine: false, // for now, make this a config var later
    };

    manifest.push(nav_item);

    zip.start_file("nav.xhtml", zip_file_options)?;

    let mut writer = EventWriter::new_with_config(&mut zip, xml_config.clone());

    writer
        .write(XmlEvent::StartDocument {
            version: xml::common::XmlVersion::Version10,
            encoding: Some("UTF-8"),
            standalone: None,
        })
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(
            XmlEvent::start_element("html")
                .ns(NS_NO_PREFIX, xhtml::NS_XHTML_URI)
                .ns(NS_EPUB_PREFIX, NS_EPUB_URI),
        )
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::start_element("head"))
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::start_element("title"))
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::characters("Table of Contents"))
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </title>
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </head>
    writer
        .write(XmlEvent::start_element("body"))
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::start_element("nav").attr(Name::prefixed("type", NS_EPUB_PREFIX), "toc"))
        .map_err(xhtml::xml_to_io_error)?;
    write_nav(&book.tree, &mut writer).map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </nav>
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </body>
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </html>

    for file in book.extra_files {
        let mut id = format!("non-md-res{}", manifest.len());

        manifest.push(ManifestItem {
            id,
            path: file.dest_path.clone(),
            media_type: file.content_type.clone(),
            properties: vec![],
            fallback: None,
            spine: false,
        });

        zip.start_file(file.dest_path.to_string_lossy(), zip_file_options)?;

        std::io::copy(&mut std::fs::File::open(&file.src_path)?, &mut zip)?;
    }

    let package = package::EpubPackage { info, manifest };

    let mut package_file = package_id;
    package_file += ".opf";
    zip.start_file(&package_file, zip_file_options)?;

    let mut writer = EventWriter::new_with_config(&mut zip, xml_config.clone());

    writer
        .write(XmlEvent::StartDocument {
            version: xml::common::XmlVersion::Version10,
            encoding: Some("UTF-8"),
            standalone: None,
        })
        .map_err(xhtml::xml_to_io_error)?;

    package
        .serialize(&mut writer)
        .map_err(xhtml::xml_to_io_error)?;

    zip.start_file("META-INF/container.xml", zip_file_options)?;

    let mut writer = EventWriter::new_with_config(&mut zip, xml_config.clone());

    writer
        .write(XmlEvent::StartDocument {
            version: xml::common::XmlVersion::Version10,
            encoding: Some("UTF-8"),
            standalone: None,
        })
        .map_err(xhtml::xml_to_io_error)?;

    writer
        .write(
            XmlEvent::start_element("container")
                .ns(NS_NO_PREFIX, NS_CONTAINER_URI)
                .attr("version", package::OCF_CONTAINER_VERSION),
        )
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::start_element("rootfiles"))
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(
            XmlEvent::start_element("rootfile")
                .attr("full-path", &package_file)
                .attr("media-type", EPUB_PACKAGE_MEDIA_TYPE),
        )
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </rootfile>
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </rootfiles>
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </container>

    zip.finish()?;
    Ok(())
}
