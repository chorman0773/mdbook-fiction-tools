use std::{
    borrow::Cow,
    io::{self, Cursor},
};
use xml::{
    name::{Name, OwnedName},
    namespace::NS_NO_PREFIX,
    writer::{EventWriter, XmlEvent},
};

use crate::bookir::{
    Alignment, Book, BookChapter, CowStr, HeadingLevel, InlineXhtml, Link, ListStyle, RichText,
    XmlNode,
};

pub fn xml_to_io_error(e: xml::writer::Error) -> std::io::Error {
    #[derive(Debug)]
    struct Xml(xml::writer::Error);

    impl core::fmt::Display for Xml {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.fmt(f)
        }
    }

    impl std::error::Error for Xml {}

    match e {
        xml::writer::Error::Io(e) => e,
        e => std::io::Error::new(std::io::ErrorKind::InvalidInput, Xml(e)),
    }
}

pub const NS_XHTML_URI: &str = "http://www.w3.org/1999/xhtml";

pub const XHTML_MEDIA: &str = "application/xhtml+xml";

pub fn write_rich_node<W: std::io::Write>(
    node: &RichText,
    writer: &mut EventWriter<W>,
) -> xml::writer::Result<()> {
    let node = node.to_xhtml();
    write_inline_node(&node, writer)
}

pub fn write_inline_node<W: std::io::Write>(node: &InlineXhtml, writer: &mut EventWriter<W>) -> xml::writer::Result<()>{
    match node {
        InlineXhtml::Node(node) => {
            match node {
                XmlNode::Block(xml_elem, rich_texts) => {
                    writer.write(xml_elem)?;

                    for rich in rich_texts{ 
                        write_rich_node(rich, writer)?;
                    }

                    writer.write(XmlEvent::end_element())
                },
                XmlNode::Inline(xml_elem) => {
                    writer.write(xml_elem)?;
                    writer.write(XmlEvent::end_element())
                },
            }
        },
        InlineXhtml::Comment(st) => writer.write(XmlEvent::comment(&st)),
        InlineXhtml::CData(st) => writer.write(XmlEvent::CData(&st)),
    }
}

pub fn write_chapter<W: std::io::Write>(
    book: &BookChapter,
    writer: &mut EventWriter<W>,
) -> xml::writer::Result<()> {
    writer.write(XmlEvent::StartDocument {
        version: xml::common::XmlVersion::Version11,
        encoding: Some("UTF-8"),
        standalone: None,
    })?;
    writer.write(XmlEvent::start_element("html").ns(NS_NO_PREFIX, NS_XHTML_URI))?;

    writer.write(XmlEvent::start_element("body"))?;
    for elem in &book.content {
        write_rich_node(elem, writer)?;
    }
    writer.write(XmlEvent::end_element())?;
    writer.write(XmlEvent::end_element())
}
