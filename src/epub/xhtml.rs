use pulldown_cmark::{CowStr, Event as MdEvent, HeadingLevel, Parser, Tag as MdTag};
use std::{
    borrow::Cow,
    io::{self, Cursor},
};
use xml::{
    name::{Name, OwnedName},
    writer::{EventWriter, XmlEvent},
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

pub fn write_md<W: std::io::Write>(
    parse: &mut Parser,
    writer: &mut EventWriter<W>,
) -> xml::writer::Result<()> {
    writer.write(XmlEvent::start_element("body"))?;

    while let Some(event) = parse.next() {
        match event {
            MdEvent::Start(tag) => match tag {
                MdTag::Paragraph => writer.write(XmlEvent::start_element("p"))?,
                MdTag::Heading(level, id, _) => {
                    let name = match level {
                        HeadingLevel::H1 => "h1",
                        HeadingLevel::H2 => "h2",
                        HeadingLevel::H3 => "h3",
                        HeadingLevel::H4 => "h4",
                        HeadingLevel::H5 => "h5",
                        HeadingLevel::H6 => "h6",
                    };
                    let mut builder = XmlEvent::start_element(name);
                    if let Some(id) = id {
                        builder = builder.attr("id", id);
                    }

                    writer.write(builder)?;
                }
                MdTag::BlockQuote => {
                    writer.write(XmlEvent::start_element("blockquote"))?;
                }
                MdTag::CodeBlock(_) => {
                    writer.write(XmlEvent::start_element("div").attr("class", "code"))?;
                }
                MdTag::List(None) => {
                    writer.write(XmlEvent::start_element("ul"))?;
                }
                MdTag::List(Some(base)) => {
                    let base = base.to_string();

                    writer.write(XmlEvent::start_element("ol").attr("start", &base))?;
                }
                MdTag::Item => writer.write(XmlEvent::start_element("li"))?,
                MdTag::FootnoteDefinition(id) => {
                    let mut st = String::from("footnote-");
                    st.push_str(&id);
                    writer.write(XmlEvent::start_element("div").attr("id", &st))?;
                }
                MdTag::Table(_) => {
                    writer.write(XmlEvent::start_element("table"))?;
                }
                MdTag::TableHead => {
                    writer.write(XmlEvent::start_element("thead"))?;
                }
                MdTag::TableRow => {
                    writer.write(XmlEvent::start_element("tr"))?;
                }
                MdTag::TableCell => {
                    writer.write(XmlEvent::start_element("td"))?;
                }
                MdTag::Emphasis => writer.write(XmlEvent::start_element("i"))?,
                MdTag::Strong => writer.write(XmlEvent::start_element("b"))?,
                MdTag::Strikethrough => writer.write(XmlEvent::start_element("s"))?,
                MdTag::Link(_, dest, _) => {
                    let dest = if !dest.contains(":") {
                        if let Some(dest) = dest.strip_suffix(".md") {
                            let mut st = dest.to_string();
                            st.push_str(".xhtml");
                            Cow::Owned(st)
                        } else {
                            Cow::Borrowed(&*dest)
                        }
                    } else {
                        Cow::Borrowed(&*dest)
                    };

                    writer.write(XmlEvent::start_element("a").attr("href", &dest))?;
                }
                MdTag::Image(_, dest, title) => {
                    let (alt_text, end) = match parse.next().unwrap() {
                        MdEvent::Text(text) => (text, false),
                        MdEvent::End(_) => (CowStr::Borrowed("no-alt"), true),
                        _ => panic!("Can only include text in an image tag"),
                    };
                    writer.write(
                        XmlEvent::start_element("img")
                            .attr("src", &dest)
                            .attr("alt", &alt_text),
                    )?;

                    if end {
                        writer.write(XmlEvent::end_element())?;
                    }
                }
            },
            MdEvent::End(event) => writer.write(XmlEvent::end_element())?,
            MdEvent::Text(text) => {
                writer.write(XmlEvent::characters(&text))?;
            }
            MdEvent::Code(text) => {
                writer.write(XmlEvent::start_element("span").attr("class", "code"))?;
                writer.write(XmlEvent::characters(&text))?;
                writer.write(XmlEvent::end_element())?;
            }
            MdEvent::Html(elem) => {
                if let Some(suffix) = elem.strip_prefix("</") {
                    let elem = suffix
                        .strip_suffix(">")
                        .ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "Not an html element")
                        })?
                        .trim();

                    writer.write(XmlEvent::end_element().name(elem))?;
                } else if let Some(suffix) = elem.strip_prefix("<!--") {
                } else {
                    let mut inner = xml::reader::EventReader::new(Cursor::new(elem.as_bytes()));
                    inner
                        .next()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?; // There's a `StartElement`

                    match inner
                        .next()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                    {
                        xml::reader::XmlEvent::StartElement {
                            name, attributes, ..
                        } => {
                            let mut event = XmlEvent::start_element(name.borrow());
                            for attr in &attributes {
                                event = event.attr(attr.name.borrow(), &attr.value);
                            }

                            writer.write(event)?;
                        }
                        xml::reader::XmlEvent::CData(string) => {
                            writer.write(XmlEvent::cdata(&string))?;
                        }
                        xml::reader::XmlEvent::Comment(string) => {
                            writer.write(XmlEvent::comment(&string))?;
                        }
                        e => panic!("Cannot process {e:?}"),
                    }

                    match inner.next() {
                        Ok(xml::reader::XmlEvent::EndElement { name }) => {
                            writer.write(XmlEvent::end_element().name(name.borrow()))?;
                        }
                        _ => {}
                    }
                }
            }
            MdEvent::FootnoteReference(a) => {
                let mut st = String::from("#footnote-");
                st.push_str(&a);
                writer.write(XmlEvent::start_element("sup"))?;
                writer.write(XmlEvent::start_element("a").attr("href", &st))?;
                writer.write(XmlEvent::characters(&a))?;
                writer.write(XmlEvent::end_element())?;
                writer.write(XmlEvent::end_element())?;
            }
            MdEvent::SoftBreak => {
                writer.write(XmlEvent::start_element("br"))?;
                writer.write(XmlEvent::end_element())?;
            }
            MdEvent::HardBreak => {
                writer.write(XmlEvent::start_element("br"))?;
                writer.write(XmlEvent::end_element())?;
                writer.write(XmlEvent::start_element("br"))?;
                writer.write(XmlEvent::end_element())?;
            }
            MdEvent::Rule => {
                writer.write(XmlEvent::start_element("hr"))?;
                writer.write(XmlEvent::end_element())?;
            }
            MdEvent::TaskListMarker(val) => {}
        }
    }

    writer.write(XmlEvent::end_element())
}
