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
    Book, BookChapter, CowStr, HeadingLevel, InlineXhtml, Link, ListStyle, RichText, XmlNode,
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
    match node {
        RichText::RawText(cow_str) => writer.write(XmlEvent::characters(cow_str)),
        RichText::Xhtml(inline_xhtml) => match inline_xhtml {
            InlineXhtml::CData(cdata) => writer.write(XmlEvent::cdata(cdata)),
            InlineXhtml::Comment(comment) => writer.write(XmlEvent::comment(comment)),
            InlineXhtml::Node(XmlNode::Block(elem_event, content)) => {
                writer.write(elem_event)?;
                for elem in content {
                    write_rich_node(elem, writer)?;
                }
                writer.write(XmlEvent::end_element())
            }
            InlineXhtml::Node(XmlNode::Inline(elem_event)) => {
                writer.write(elem_event)?;
                writer.write(XmlEvent::end_element())
            }
        },
        RichText::Stylised(attributes, elems) => {
            let mut steps = 0;

            if attributes.strikethrough {
                steps += 1;
                writer.write(XmlEvent::start_element("s"))?;
            }
            if attributes.underline {
                steps += 1;
                writer.write(XmlEvent::start_element("u"))?;
            }
            if attributes.bold {
                steps += 1;
                writer.write(XmlEvent::start_element("b"))?;
            }
            if attributes.italics {
                steps += 1;
                writer.write(XmlEvent::start_element("i"))?;
            }

            for elem in elems {
                write_rich_node(elem, writer)?;
            }

            for _ in 0..steps {
                writer.write(XmlEvent::end_element())?;
            }
            Ok(())
        }
        RichText::Paragraph(elems) => {
            writer.write(XmlEvent::start_element("p"))?;
            for elem in elems {
                write_rich_node(elem, writer)?;
            }
            writer.write(XmlEvent::end_element())
        }
        RichText::InlineCode(code) => {
            writer.write(XmlEvent::start_element("code"))?;
            writer.write(XmlEvent::cdata(code))?;
            writer.write(XmlEvent::end_element())
        }
        RichText::CodeBlock(code) => {
            writer.write(
                XmlEvent::start_element("div")
                    .attr("style", "font-family:monospace;background-color: #c9c9c9;"),
            )?;
            writer.write(XmlEvent::cdata(&code.content))?;
            writer.write(XmlEvent::end_element())
        }
        RichText::InternalLink(link) => match link {
            Link::Text {
                title: _,
                elems,
                dest_url,
            } => {
                let link = if let Some(prefix) = dest_url.strip_suffix(".md") {
                    CowStr::from(format!("{prefix}.xhtml"))
                } else {
                    dest_url.into()
                };

                writer.write(XmlEvent::start_element("a").attr("href", &link))?;
                for elem in elems {
                    write_rich_node(elem, writer)?;
                }
                writer.write(XmlEvent::end_element())
            }
            Link::Footnote(id) => todo!(),
        },
        RichText::ExternalLink(link) => match link {
            Link::Text {
                title: _,
                elems,
                dest_url,
            } => {
                let link = dest_url.clone();

                writer.write(XmlEvent::start_element("a").attr("href", &link))?;
                for elem in elems {
                    write_rich_node(elem, writer)?;
                }
                writer.write(XmlEvent::end_element())
            }
            Link::Footnote(id) => unreachable!("External Link to a footnote not possible"),
        },
        RichText::InternalImage(link) | RichText::ExternalImage(link) => match link {
            Link::Text {
                title: _,
                elems,
                dest_url,
            } => {
                let link = dest_url.clone();

                let mut alt = String::new();

                for elem in elems {
                    match elem {
                        RichText::RawText(raw) => alt.push_str(raw),
                        r => panic!("Can't include non-raw alt text in an image {r:?}"),
                    }
                }

                writer.write(
                    XmlEvent::start_element("img")
                        .attr("src", &link)
                        .attr("alt", &alt),
                )?;
                for elem in elems {
                    write_rich_node(elem, writer)?;
                }
                writer.write(XmlEvent::end_element())
            }
            Link::Footnote(id) => unreachable!("External Link to a footnote not possible"),
        },
        RichText::Heading(heading) => {
            let start = match heading.level {
                HeadingLevel::H1 => XmlEvent::start_element("h1"),
                HeadingLevel::H2 => XmlEvent::start_element("h2"),
                HeadingLevel::H3 => XmlEvent::start_element("h3"),
                HeadingLevel::H4 => XmlEvent::start_element("h4"),
                HeadingLevel::H5 => XmlEvent::start_element("h5"),
                HeadingLevel::H6 => XmlEvent::start_element("h6"),
            };

            writer.write(start.attr("id", &heading.id))?;
            writer.write(XmlEvent::characters(&heading.text))?;
            writer.write(XmlEvent::end_element())
        }
        RichText::TextBreak(break_type) => match break_type {
            crate::bookir::BreakType::Rule => {
                writer.write(XmlEvent::start_element("hr"))?;
                writer.write(XmlEvent::end_element())
            }
            crate::bookir::BreakType::SoftLine | crate::bookir::BreakType::HardLine => {
                writer.write(XmlEvent::start_element("br"))?;
                writer.write(XmlEvent::end_element())
            }
        },
        RichText::List(list) => {
            match list.list_style {
                ListStyle::Ordered(n) => {
                    writer.write(XmlEvent::start_element("ol").attr("start", &format!("{n}")))?
                }
                ListStyle::Unordered => writer.write(XmlEvent::start_element("ul"))?,
            }

            for item in &list.elems {
                writer.write(XmlEvent::start_element("li"))?;
                for elem in &item.0 {
                    write_rich_node(elem, writer)?;
                }
                writer.write(XmlEvent::end_element())?;
            }
            writer.write(XmlEvent::end_element())
        }
        RichText::BlockQuote(vec) => {
            writer.write(XmlEvent::start_element("bq"))?;
            for elem in vec {
                write_rich_node(elem, writer)?;
            }
            writer.write(XmlEvent::end_element())
        }
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
