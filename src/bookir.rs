use std::io::{self, Stdin};
use std::process::Stdio;
use std::{
    borrow::{Borrow, Cow},
    num::NonZero,
    ops::ControlFlow,
    path::{Path, PathBuf},
    process::Command,
};

use indexmap::IndexMap;
use ::xml::{name::Name, reader::XmlEvent, EventReader};
use mdbook_core::{book::{BookItems, BookItem}, };
use nav::NavTree;
use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel as MdHeadingLevel, InlineStr, LinkType, Parser, Tag, TagEnd,
};
use serde::{Deserialize, Serialize};

use crate::helpers::{self, name_to_id};

#[cfg(feature = "math")]
pub mod math;

pub mod nav;
pub mod render;
pub mod str;
pub mod xml;

use xml::XmlElem;

pub use str::CowStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XmlNode<'a> {
    Block(XmlElem, Vec<RichText<'a>>),
    Inline(XmlElem),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InlineXhtml<'a> {
    Node(XmlNode<'a>),
    Comment(CowStr<'a>),
    CData(CowStr<'a>),
    Text(CowStr<'a>),
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Attributes {
    pub bold: bool,
    pub italics: bool,
    pub underline: bool,
    pub strikethrough: bool,

    #[doc(hidden)]
    #[serde(skip)]
    pub __non_exhaustive: (),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Link<'a> {
    Text {
        title: CowStr<'a>,
        elems: Vec<RichText<'a>>,
        dest_url: CowStr<'a>,
    },
    Footnote(CowStr<'a>),
}

impl<'a> Link<'a> {
    fn to_linkref(&self) -> (Vec<RichText<'a>>, CowStr<'a>) {
        match self {
            Link::Text { elems, dest_url, .. } => {
                (elems.clone(), dest_url.clone())
            },
            Link::Footnote(fnref) => {
                let id = format!("#fn-{}", name_to_id(fnref));

                (vec![RichText::RawText(fnref.clone())], CowStr::Boxed(id.into_boxed_str()))
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock<'a> {
    pub lang: CowStr<'a>,
    pub content: CowStr<'a>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table<'a> {
    pub align: Vec<Alignment>,
    pub head: Option<TableRow<'a>>,
    pub body: Vec<TableRow<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRow<'a> {
    pub elems: Vec<RichText<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RichText<'a> {
    RawText(CowStr<'a>),
    Comment(CowStr<'a>),
    Xhtml(InlineXhtml<'a>),
    Stylised(Attributes, Vec<RichText<'a>>),
    Paragraph(Vec<RichText<'a>>),
    InlineCode(CowStr<'a>),
    CodeBlock(CodeBlock<'a>),
    BlockQuote(Vec<RichText<'a>>),
    InternalLink(Link<'a>),
    ExternalLink(Link<'a>),
    InternalImage(Link<'a>),
    ExternalImage(Link<'a>),
    Heading(Heading<'a>),
    TextBreak(BreakType),
    List(List<'a>),
    Table(Table<'a>),
    #[cfg(feature = "math")]
    MathBlock(math::Math<'a>),
    #[cfg(feature = "math")]
    InlineMath(math::Math<'a>),
    Macro(CowStr<'a>),
}

// Implementation detail of `normalize`
enum NormalizationResult {
    Simple,
    Link,
    List,
    Complex,
    Html,
}

impl<'a> RichText<'a> {
    /// Normalizes [`RichText`] so that it can be emitted as markdown
    pub fn normalize(&mut self) {
        match self.normalize_inner() {
            NormalizationResult::Html => {
                *self = RichText::Xhtml(self.to_xhtml());
            }
            _ => {}
        }
    }

    /// Converts a [`RichText`] node to [`InlineXhtml`] that only contains xhtml nodes (and raw text)
    pub fn to_xhtml(&self) -> InlineXhtml<'a> {
        fn push_recurse_node<'a>(node: &mut Option<XmlNode<'a>>, rich: &mut Vec<RichText<'a>>, elem: XmlElem) {
            if let Some(node) = node.take() {
                rich.push(RichText::Xhtml(InlineXhtml::Node(node)));
            }

            *node = Some(XmlNode::Block(elem, core::mem::take(rich)));
        }
        match self {
            RichText::RawText(text) => InlineXhtml::Text(text.clone()),
            RichText::Comment(comment) => InlineXhtml::Comment(comment.clone()),
            RichText::Xhtml(xhtml) => {
                match xhtml {
                    InlineXhtml::Node(node) => {
                        let node = match node {
                            XmlNode::Block(elem, rich_texts) => {
                                let rich = rich_texts.iter().map(|v| v.to_xhtml()).map(RichText::Xhtml).collect();

                                XmlNode::Block(elem.clone(), rich)
                            },
                            XmlNode::Inline(elem) => XmlNode::Inline(elem.clone()),
                        };

                        InlineXhtml::Node(node)
                    },
                    InlineXhtml::Comment(_) |
                    InlineXhtml::CData(_) |
                    InlineXhtml::Text(_) => xhtml.clone(),
                }
            },
            RichText::Stylised(attributes, rich_texts) => {
                let mut rich = rich_texts.iter().map(|v| v.to_xhtml()).map(RichText::Xhtml).collect::<Vec<_>>();

                let mut node = None;

                if attributes.bold {
                    push_recurse_node(&mut node, &mut rich, XmlElem::simple("b"));
                }

                if attributes.italics {
                    push_recurse_node(&mut node, &mut rich, XmlElem::simple("i"));
                }

                if attributes.strikethrough {
                    push_recurse_node(&mut node, &mut rich, XmlElem::simple("s"));
                }

                if attributes.underline {
                    push_recurse_node(&mut node, &mut rich, XmlElem::simple("u"));
                }

                let node = node.expect("At least one attribute must be present");

                InlineXhtml::Node(node)
            },
            RichText::Paragraph(elems) => {
                let rich = elems.iter().map(|v| v.to_xhtml()).map(RichText::Xhtml).collect::<Vec<_>>();
                let node = XmlNode::Block(XmlElem::simple("p"), rich);

                InlineXhtml::Node(node)
            },
            RichText::InlineCode(cow_str) => {
                let text = vec![RichText::RawText(cow_str.clone())];

                let node = XmlNode::Block(XmlElem::simple("code"), text);

                InlineXhtml::Node(node)
            },
            RichText::CodeBlock(code_block) => {
                let class = if code_block.lang.is_empty() {
                    "code-block".to_string()
                } else {
                    format!("code-block code-{lang}", lang = name_to_id(&code_block.lang))
                };

                let text = vec![RichText::RawText(code_block.content.clone())];

                let mut elem = XmlElem::simple("div");
                elem.attrs.insert("class".to_string(), class);

                let node = XmlNode::Block(elem, text);

                InlineXhtml::Node(node)
            },
            RichText::BlockQuote(elems) => {
                let rich = elems.iter().map(|v| v.to_xhtml()).map(RichText::Xhtml).collect::<Vec<_>>();
                let node = XmlNode::Block(XmlElem::simple("bq"), rich);

                InlineXhtml::Node(node)
            },
            RichText::InternalLink(link) |
            RichText::ExternalLink(link) => {
                let (text, dest) = link.to_linkref();

                let text = text.iter().map(|v| v.to_xhtml()).map(RichText::Xhtml).collect::<Vec<_>>();

                let mut elem = XmlElem::simple("a");
                elem.attrs.insert("href".to_string(), dest.into());

                let node = XmlNode::Block(elem, text);

                InlineXhtml::Node(node)
            },
            RichText::InternalImage(link) |
            RichText::ExternalImage(link) => {
                let (text, dest) = link.to_linkref();

                let text = text.iter().map(|v| v.to_xhtml()).map(RichText::Xhtml).collect::<Vec<_>>();

                let mut elem = XmlElem::simple("img");
                elem.attrs.insert("src".to_string(), dest.into());

                let node = XmlNode::Block(elem, text);

                InlineXhtml::Node(node)
            },
            RichText::Heading(heading) => {
                let level = match heading.level {
                    HeadingLevel::H1 => "h1",
                    HeadingLevel::H2 => "h2",
                    HeadingLevel::H3 => "h3",
                    HeadingLevel::H4 => "h4",
                    HeadingLevel::H5 => "h5",
                    HeadingLevel::H6 => "h6",
                };

                let mut elem = XmlElem::simple(level);

                elem.attrs.insert("id".to_string(), heading.id.clone().into());

                let text = vec![RichText::RawText(heading.text.clone())];

                let node = XmlNode::Block(elem, text);

                InlineXhtml::Node(node)
            },
            RichText::TextBreak(break_type) => {
                let br = match break_type {
                    BreakType::Rule => "hr",
                    BreakType::SoftLine |
                    BreakType::HardLine => "br",
                };

                let elem = XmlElem::simple(br);

                InlineXhtml::Node(XmlNode::Inline(elem))
            },
            RichText::List(list) => {
                let root_elem = match list.list_style {
                    ListStyle::Unordered => XmlElem::simple("ul"),
                    ListStyle::Ordered(start) => {
                        let mut elem = XmlElem::simple("ol");

                        if start != 1 {
                            elem.attrs.insert("start".to_string(), format!("{start}"));
                        }

                        elem
                    },
                };

                let rich = list.elems.iter().map(|v| {
                    let rich = v.0.iter().map(|v| v.to_xhtml()).map(RichText::Xhtml).collect();

                    let elem = XmlElem::simple("li");

                    XmlNode::Block(elem, rich)
                })
                .map(InlineXhtml::Node)
                .map(RichText::Xhtml)
                .collect();

                let node = XmlNode::Block(root_elem, rich);

                InlineXhtml::Node(node)
            },
            RichText::Table(table) => todo!(),
            RichText::Macro(macro_name) => InlineXhtml::Comment(macro_name.clone()), // Macros need special support, so just emit a comment for now
        }
    }

    fn normalize_inner(&mut self) -> NormalizationResult {
        match self {
            RichText::RawText(_) | RichText::InlineCode(_) | RichText::Comment(_) => NormalizationResult::Simple,
            RichText::Xhtml(_) => {
                NormalizationResult::Html
            }, // TODO: We can convert some xhtml back into RichText
            RichText::Stylised(attributes, rich_texts) => {
                match &mut **rich_texts {
                    [single] => {
                        match single.normalize_inner() {
                            NormalizationResult::Simple => {
                                match single {
                                    RichText::Stylised(inner_attributes, inner_rich) => {
                                        attributes.bold |= inner_attributes.bold;
                                        attributes.italics |= inner_attributes.italics;
                                        attributes.strikethrough |= inner_attributes.strikethrough;
                                        attributes.underline |= inner_attributes.underline;
                                        *rich_texts = core::mem::take(inner_rich);
                                        
                                    },
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    a => {
                        for elem in rich_texts {
                            match elem.normalize_inner() {
                                NormalizationResult::Simple | NormalizationResult::Link => {},
                                
                                NormalizationResult::List |
                                NormalizationResult::Complex |
                                NormalizationResult::Html => {
                                    return NormalizationResult::Html
                                },
                            }
                        }
                    }
                }
                NormalizationResult::Simple
            },
            RichText::Paragraph(rich_texts) => {
                for text in rich_texts {
                    match text.normalize_inner() {
                        NormalizationResult::Simple | NormalizationResult::Complex | NormalizationResult::Link | NormalizationResult::List => {}
                        NormalizationResult::Html => {
                            *text = RichText::Xhtml(text.to_xhtml());
                        }
                    }
                }
                NormalizationResult::Complex
            },
            
            RichText::BlockQuote(rich_texts) => {
                for text in rich_texts {
                    match text.normalize_inner() {
                        NormalizationResult::Simple | NormalizationResult::Complex | NormalizationResult::Link | NormalizationResult::List => {}
                        NormalizationResult::Html => {
                            *text = RichText::Xhtml(text.to_xhtml());
                        }
                    }
                }
                NormalizationResult::Complex
            },
            RichText::InternalLink(link) |
            RichText::ExternalLink(link) |
            RichText::InternalImage(link) |
            RichText::ExternalImage(link) => {
                match link {
                    Link::Text { elems, .. } => {
                        for elem in elems {
                            match elem.normalize_inner() {
                                NormalizationResult::Simple | NormalizationResult::Link => {}
                                NormalizationResult::Complex |
                                NormalizationResult::List |
                                NormalizationResult::Html => return NormalizationResult::Html,
                            }
                        }
                        NormalizationResult::Link
                    },
                    Link::Footnote(cow_str) => NormalizationResult::Link,
                }
            },
            
            
            RichText::List(list) => {
                for elem in &mut list.elems {
                    for rich in &mut elem.0 {
                        match rich.normalize_inner() {
                            NormalizationResult::Simple | NormalizationResult::Link | NormalizationResult::List => {}
                            NormalizationResult::Complex | NormalizationResult::Html => return NormalizationResult::Html
                        }
                    }
                }

                NormalizationResult::List
            },
            
            RichText::Table(table) => {
                for row in table.head.iter_mut().chain(&mut table.body){
                    for elem in &mut row.elems {
                        match elem.normalize_inner() {
                            NormalizationResult::Simple | NormalizationResult::Link => {}
                            NormalizationResult::List| NormalizationResult::Complex | NormalizationResult::Html => return NormalizationResult::Html
                        }
                    }
                }
                NormalizationResult::Complex
            },
            RichText::Heading(_) |
            RichText::CodeBlock(_) |
            RichText::TextBreak(_) => NormalizationResult::Complex,
            RichText::Macro(name) => {
                NormalizationResult::Simple
            },
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListStyle {
    Unordered,
    #[serde(untagged)]
    Ordered(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem<'a>(pub Vec<RichText<'a>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List<'a> {
    pub list_style: ListStyle,
    pub elems: Vec<ListItem<'a>>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakType {
    Rule,
    SoftLine,
    HardLine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading<'a> {
    pub level: HeadingLevel,
    pub text: CowStr<'a>,
    pub id: CowStr<'a>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeadingLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct RichTextOptions {
    #[cfg_attr(not(feature = "math"), serde(skip))]
    pub math: bool,

    #[doc(hidden)]
    pub __non_exhausitve: (),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum EndMarker<'a> {
    MdTag(TagEnd),
    XhtmlTag(CowStr<'a>),
}

#[derive(Debug)]
pub struct RichTextParser<'a>(Parser<'a>);

impl<'a> RichTextParser<'a> {
    pub fn new(text: &'a str, options: RichTextOptions) -> Self {
        let mut md_options = pulldown_cmark::Options::ENABLE_TABLES
            | pulldown_cmark::Options::ENABLE_STRIKETHROUGH
            | pulldown_cmark::Options::ENABLE_GFM
            | pulldown_cmark::Options::ENABLE_HEADING_ATTRIBUTES;
        if options.math {
            md_options |= pulldown_cmark::Options::ENABLE_MATH;
        }

        Self(Parser::new_ext(text, md_options))
    }
}

impl<'a> RichTextParser<'a> {
    fn next_primitive(&mut self) -> Option<Result<RichText<'a>, Event<'a>>> {
        Some(match self.0.next()? {
            e @ (Event::Start(_) | Event::End(_)) => Err(e),
            Event::Text(text) => {
                if let Some(next) =  text.trim().strip_prefix("!{#").and_then(|s| s.strip_suffix("}")) {
                    let name = CowStr::Borrowed(next.trim()).into_static();

                    Ok(RichText::Macro(name))
                } else {
                    Ok(RichText::RawText(text.into()))
                }
            },
            Event::Code(code) => Ok(RichText::InlineCode(code.into())),
            #[cfg(feature = "math")]
            Event::InlineMath(tex) => todo!("latex {tex}"),
            #[cfg(feature = "math")]
            Event::DisplayMath(tex) => todo!("latex {tex}"),
            #[cfg(not(feature = "math"))]
            Event::InlineMath(_) | Event::DisplayMath(_) => unreachable!("No math support"),
            Event::InlineHtml(html) | Event::Html(html) => {
                if let Some(comment) = html.strip_prefix("<!--") {
                    let comment_body = comment.strip_suffix("-->")?;
                    Ok(RichText::Xhtml(InlineXhtml::Comment(
                        CowStr::Borrowed(comment_body).into_static(),
                    )))
                } else if let Some(cdata) = html.strip_prefix("<![CDATA[") {
                    let cdata_body = cdata.strip_suffix("]]>")?;
                    Ok(RichText::Xhtml(InlineXhtml::CData(
                        CowStr::Borrowed(cdata_body).into_static(),
                    )))
                } else {
                    Err(Event::InlineHtml(html))
                }
            }
            Event::FootnoteReference(id) => Ok(RichText::InternalLink(Link::Footnote(id.into()))),
            Event::SoftBreak => Ok(RichText::TextBreak(BreakType::SoftLine)),
            Event::HardBreak => Ok(RichText::TextBreak(BreakType::HardLine)),
            Event::Rule => Ok(RichText::TextBreak(BreakType::Rule)),
            Event::TaskListMarker(_) => todo!("checkbox"),
        })
    }

    fn next_elem(&mut self) -> Option<ControlFlow<EndMarker<'a>, RichText<'a>>> {
        match self.next_primitive()? {
            Ok(elem) => Some(ControlFlow::Continue(elem)),
            Err(Event::Start(Tag::HtmlBlock)) | Err(Event::End(TagEnd::HtmlBlock)) => {
                self.next_elem()
            }
            Err(Event::End(tag)) => Some(ControlFlow::Break(EndMarker::MdTag(tag))),
            Err(Event::Start(tag)) => self.handle_tag(tag).map(ControlFlow::Continue),
            Err(Event::InlineHtml(html)) => {
                if let Some(elem) = html.strip_prefix("</") {
                    let elem = elem.strip_suffix(">").expect("expected valid xml").trim();

                    Some(ControlFlow::Break(EndMarker::XhtmlTag(
                        CowStr::Borrowed(elem).into_static(),
                    )))
                } else {
                    self.handle_html(html.into()).map(ControlFlow::Continue)
                }
            }
            Err(e) => unimplemented!("Non-primitive tag {e:?}"),
        }
    }

    fn to_end(&mut self) -> Option<(Vec<RichText<'a>>, EndMarker<'a>)> {
        let mut elems = Vec::new();

        loop {
            match self.next_elem()? {
                ControlFlow::Continue(elem) => elems.push(elem),
                ControlFlow::Break(marker) => break Some((elems, marker)),
            }
        }
    }

    fn handle_html(&mut self, blob: CowStr<'a>) -> Option<RichText<'a>> {
        let mut reader = EventReader::from_str(&blob);

        let elem = match reader.next().expect("inline xhtml error") {
            XmlEvent::StartDocument { .. } => reader.next().expect("inline xhtml error"),
            e => e,
        };

        let end_name = match reader.next() {
            Ok(XmlEvent::EndElement { name }) => Some(name),
            _ => None,
        };

        let elem = match elem {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                let elem = XmlElem {
                    name: name.to_string(),
                    attrs: attributes
                        .into_iter()
                        .map(|a| (a.name.to_string(), a.value))
                        .collect(),
                };
                if let Some(end) = &end_name {
                    assert_eq!(elem.name, end.to_string(), "Invalid xhtml tag {name:?}");
                    return Some(RichText::Xhtml(InlineXhtml::Node(XmlNode::Inline(elem))));
                }
                elem
            }
            e => panic!("Unexpected inline xhtml {e:?}"),
        };
        let (elems, end) = self.to_end()?;

        match end {
            EndMarker::XhtmlTag(tag) => {
                assert_eq!(tag, elem.name);
                Some(RichText::Xhtml(InlineXhtml::Node(XmlNode::Block(
                    elem, elems,
                ))))
            }
            _ => unreachable!(),
        }
    }

    fn handle_tag(&mut self, tag: Tag<'a>) -> Option<RichText<'a>> {
        match tag {
            Tag::Paragraph => {
                let (elems, _) = self.to_end()?;

                Some(RichText::Paragraph(elems))
            }
            Tag::Heading { level, id, .. } => {
                let level = match level {
                    MdHeadingLevel::H1 => HeadingLevel::H1,
                    MdHeadingLevel::H2 => HeadingLevel::H2,
                    MdHeadingLevel::H3 => HeadingLevel::H3,
                    MdHeadingLevel::H4 => HeadingLevel::H4,
                    MdHeadingLevel::H5 => HeadingLevel::H5,
                    MdHeadingLevel::H6 => HeadingLevel::H6,
                };

                let mut text = String::new();
                for i in (&mut self.0).take_while(|r| !matches!(r, Event::End(TagEnd::Heading(_))))
                {
                    match i {
                        Event::Text(t) => text.push_str(&t),
                        _ => unimplemented!("Rich Text in heading"),
                    }
                }

                let id = id.map_or_else(
                    || {
                        let id = helpers::name_to_id(&text);

                        CowStr::Boxed(id.into_boxed_str())
                    },
                    Into::into,
                );

                let text = CowStr::Boxed(text.into_boxed_str());
                Some(RichText::Heading(Heading { level, text, id }))
            }
            Tag::BlockQuote(_) => {
                let (elems, _) = self.to_end()?;

                Some(RichText::BlockQuote(elems))
            }
            Tag::CodeBlock(code_block_kind) => {
                let lang = match code_block_kind {
                    CodeBlockKind::Fenced(lang) => lang.into(),
                    CodeBlockKind::Indented => CowStr::Borrowed(""),
                };

                let mut text = String::new();

                loop {
                    match self.0.next()? {
                        Event::Text(c) => text.push_str(&c),
                        Event::End(TagEnd::CodeBlock) => break,
                        e => unreachable!("Unexpected event {e:?}"),
                    }
                }

                Some(RichText::CodeBlock(CodeBlock {
                    lang,
                    content: CowStr::Boxed(text.into_boxed_str()),
                }))
            }
            Tag::HtmlBlock => unreachable!(),
            Tag::List(n) => {
                let style = n.map_or(ListStyle::Unordered, ListStyle::Ordered);
                let mut elems = Vec::new();
                loop {
                    match self.0.next()? {
                        Event::Start(Tag::Item) => {
                            let (content, _) = self.to_end()?;
                            elems.push(ListItem(content))
                        }
                        Event::End(TagEnd::List(_)) => {
                            break Some(RichText::List(List {
                                list_style: style,
                                elems,
                            }))
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Tag::Item => unreachable!(),
            Tag::FootnoteDefinition(cow_str) => todo!(),
            Tag::DefinitionList => todo!(),
            Tag::DefinitionListTitle => todo!(),
            Tag::DefinitionListDefinition => todo!(),
            Tag::Table(_) => todo!(),
            Tag::TableHead => todo!(),
            Tag::TableRow => todo!(),
            Tag::TableCell => todo!(),
            Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            } => {
                let is_internal = dest_url.find("://").is_none();
                let (elems, _) = self.to_end()?;
                let link = Link::Text {
                    title: title.into(),
                    elems,
                    dest_url: dest_url.into(),
                };

                if is_internal {
                    Some(RichText::InternalLink(link))
                } else {
                    Some(RichText::ExternalLink(link))
                }
            }
            Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            } => {
                let is_internal = dest_url.find("://").is_none();
                let (elems, _) = self.to_end()?;
                let link = Link::Text {
                    title: title.into(),
                    elems,
                    dest_url: dest_url.into(),
                };

                if is_internal {
                    Some(RichText::InternalImage(link))
                } else {
                    Some(RichText::ExternalImage(link))
                }
            }
            Tag::MetadataBlock(metadata_block_kind) => unimplemented!("Metadata should not parse"),
            Tag::Strong | Tag::Emphasis | Tag::Strikethrough => {
                let (mut elems, end) = self.to_end()?;
                let (mut style, elems) = match &mut elems[..] {
                    [RichText::Stylised(style, elems)] => (*style, core::mem::take(elems)),
                    _ => (Attributes::default(), elems),
                };

                match end {
                    EndMarker::MdTag(TagEnd::Strong) => style.bold = true,
                    EndMarker::MdTag(TagEnd::Emphasis) => style.italics = true,
                    EndMarker::MdTag(TagEnd::Strikethrough) => style.strikethrough = true,
                    _ => unreachable!(),
                }

                Some(RichText::Stylised(style, elems))
            }
        }
    }
}

impl<'a> Iterator for RichTextParser<'a> {
    type Item = RichText<'a>;

    fn next(&mut self) -> Option<RichText<'a>> {
        match self.next_elem()? {
            ControlFlow::Continue(elem) => Some(elem),
            ControlFlow::Break(end) => panic!("Unexpected ending tag {end:?}"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BookChapter<'a> {
    pub src_path: Cow<'a, Path>,
    pub dest_path: Cow<'a, Path>,
    pub content: Vec<RichText<'a>>,
    pub dirty: bool,
}

impl<'a> BookChapter<'a> {
    pub fn from_chapter(ch: &'a mdbook_core::book::Chapter, opts: RichTextOptions) -> Option<Self> {
        let src_path = ch.source_path.as_ref()?;
        let dest_path = ch.path.as_ref()?;
        let content = RichTextParser::new(&ch.content, opts).collect();

        Some(Self {
            src_path: Cow::Borrowed(src_path),
            dest_path: Cow::Borrowed(dest_path),
            content,
            dirty: false,
        })
    }
}

#[derive(Clone, Serialize)]
pub struct ExtraItem {
    pub src_path: PathBuf,
    pub dest_path: PathBuf,
    pub content_type: Cow<'static, str>,
}

impl core::fmt::Debug for ExtraItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.src_path.fmt(f)?;
        f.write_str(":")?;
        self.dest_path.fmt(f)?;
        f.write_str(" (")?;
        f.write_str(&self.content_type)?;
        f.write_str(")")
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Book<'a> {
    pub title: &'a str,
    pub tree: NavTree<'a>,
    pub extra_files: &'a [ExtraItem],
    pub authors: &'a [&'a str],
    pub id: &'a str,
}

impl<'a> Book<'a> {
    pub fn build<A: Borrow<BookItem>>(
        title: &'a str,
        items: &'a [A],
        opts: RichTextOptions,
        extra_files: &'a [ExtraItem],
        authors: &'a [&'a str],
        id: &'a str,
    ) -> Book<'a> {
        Book {
            title,
            tree: NavTree::from_items(items, opts),
            extra_files,
            authors,
            id,
        }
    }
}
