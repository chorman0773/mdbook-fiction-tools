use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use mdbook::{book::Chapter, BookItem};
use pulldown_cmark::{Options, Parser};
use xml::{
    name::Name,
    namespace::{Namespace, NS_NO_PREFIX, NS_XML_PREFIX, NS_XML_URI},
    writer::XmlEvent,
    EmitterConfig, EventWriter,
};
use zip::write::{FileOptions, ZipWriter};

use crate::{
    epub::{
        info::NS_CONTAINER_URI,
        package::{ItemProperty, ManifestItem, EPUB_PACKAGE_MEDIA_TYPE},
    },
    helpers::{media_type_from_file, name_to_id, visit_chapters},
};

use self::{
    info::EpubFileInfo,
    nav::{NavHeading, NavNode, NavTree},
};

pub mod config;
pub mod info;
pub mod nav;
pub mod package;
pub mod style;
pub mod xhtml;

pub const NS_EPUB_PREFIX: &str = "epub";
pub const NS_EPUB_URI: &str = "http://www.idpf.org/2007/ops";

pub fn write_epub<
    'a,
    W: std::io::Write + std::io::Seek,
    I: IntoIterator<Item = &'a BookItem> + Clone,
    E: IntoIterator,
>(
    writer: W,
    chapters: I,
    info: EpubFileInfo,
    package_id: String,
    extra_files: E,
    root: &Path,
) -> std::io::Result<()>
where
    E::Item: AsRef<Path>,
{
    use std::io::Write;
    let md_options = Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let zip_file_options = FileOptions::default().unix_permissions(0o644);
    let xml_config = EmitterConfig::new();
    let mut zip = ZipWriter::new(writer);

    zip.set_comment(&info.title);

    zip.start_file(
        "mimetype",
        FileOptions::default().compression_method(zip::CompressionMethod::Stored),
    )?;
    write!(zip, "application/epub+zip")?;

    let mut manifest = Vec::new();

    let nav_item = ManifestItem {
        id: format!("nav-toc"),
        path: PathBuf::from("nav.xhtml"),
        media_type: Cow::Borrowed(xhtml::XHTML_MEDIA),
        properties: vec![ItemProperty::Nav],
        fallback: None,
        spine: false, // for now, make this a config var later
    };

    manifest.push(nav_item);

    let (mut nav_tree, outstanding) = visit_chapters(
        chapters.clone(),
        |item, (nav_tree, outstanding)| {
            match item {
                BookItem::Separator => {
                    if core::mem::take(outstanding.last_mut().unwrap()) {
                        nav_tree.push(NavNode {
                            heading: NavHeading::End,
                            children: None,
                        });
                    }
                }
                BookItem::PartTitle(part) => {
                    if core::mem::take(outstanding.last_mut().unwrap()) {
                        nav_tree.push(NavNode {
                            heading: NavHeading::End,
                            children: None,
                        });
                    }
                    nav_tree.push(NavNode {
                        heading: NavHeading::Heading(part.clone()),
                        children: None,
                    });
                    *outstanding.last_mut().unwrap() = true;
                }
                BookItem::Chapter(c) => {
                    if let Some(file) = &c.path {
                        use core::fmt::Write as _;
                        outstanding.push(false);

                        let mut file = file.clone();
                        file.set_extension("xhtml");

                        zip.start_file(file.to_string_lossy(), zip_file_options)?;

                        nav_tree.push(NavNode {
                            heading: NavHeading::Chapter(c.name.clone(), file.clone()),
                            children: None,
                        });

                        let mut id = name_to_id(&c.name);
                        write!(id, "{}", manifest.len()).unwrap();

                        manifest.push(ManifestItem {
                            id,
                            path: file,
                            media_type: Cow::Borrowed(xhtml::XHTML_MEDIA),
                            properties: vec![],
                            fallback: None,
                            spine: true,
                        });

                        let mut md_parser = Parser::new_ext(&c.content, md_options);

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
                            .write(XmlEvent::characters(&c.name))
                            .map_err(xhtml::xml_to_io_error)?;
                        writer
                            .write(XmlEvent::end_element())
                            .map_err(xhtml::xml_to_io_error)?; // </title>
                        writer
                            .write(XmlEvent::end_element())
                            .map_err(xhtml::xml_to_io_error)?; // </head>
                        xhtml::write_md(&mut md_parser, &mut writer)
                            .map_err(xhtml::xml_to_io_error)?;
                        writer
                            .write(XmlEvent::end_element())
                            .map_err(xhtml::xml_to_io_error)?; // </html>
                    } else {
                        nav_tree.push(NavNode {
                            heading: NavHeading::Heading(c.name.clone() + " (Draft Chapter)"),
                            children: None,
                        });
                    }
                }
            }
            Ok(())
        },
        |(nav_tree, outstanding)| {
            if outstanding.pop().unwrap() {
                nav_tree.push(NavNode {
                    heading: NavHeading::End,
                    children: None,
                });
            }
            nav_tree.push(NavNode {
                heading: NavHeading::End,
                children: None,
            });
        },
        (NavTree::new(), vec![false]),
    )
    .unwrap();

    if outstanding[0] {
        nav_tree.push(NavNode {
            heading: NavHeading::End,
            children: None,
        });
    }

    nav_tree.treeify();

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
    nav_tree
        .write_ol(&mut writer)
        .map_err(xhtml::xml_to_io_error)?;
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </nav>
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </body>
    writer
        .write(XmlEvent::end_element())
        .map_err(xhtml::xml_to_io_error)?; // </html>

    for file in extra_files {
        let file = file.as_ref();
        let inner_path = file.strip_prefix(root).unwrap();

        let path_str = inner_path.to_str().unwrap();

        let mut id = format!("non-md-res{}", manifest.len());

        let media_ty = media_type_from_file(&file);

        manifest.push(ManifestItem {
            id,
            path: inner_path.to_path_buf(),
            media_type: media_ty,
            properties: vec![],
            fallback: None,
            spine: false,
        });

        zip.start_file(path_str, zip_file_options)?;

        std::io::copy(&mut std::fs::File::open(file)?, &mut zip)?;
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
