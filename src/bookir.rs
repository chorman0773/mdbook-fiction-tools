use std::{
    borrow::{Borrow, Cow},
    num::NonZero,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use mdbook::{book::BookItems, BookItem};
use nav::NavTree;
pub use pulldown_cmark::CowStr;
use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel as MdHeadingLevel, LinkType, Parser, Tag, TagEnd,
};
use xml::{name::Name, reader::XmlEvent, EventReader};

use crate::helpers;

pub mod nav;

#[derive(Debug, Clone)]
pub enum XmlNode<'a> {
    Block(XmlEvent, Vec<RichText<'a>>),
    Inline(XmlEvent),
}

#[derive(Debug, Clone)]
pub enum InlineXhtml<'a> {
    Node(XmlNode<'a>),
    Comment(CowStr<'a>),
    CData(CowStr<'a>),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Attributes {
    pub bold: bool,
    pub italics: bool,
    pub underline: bool,
    pub strikethrough: bool,

    #[doc(hidden)]
    pub __non_exhaustive: (),
}

#[derive(Debug, Clone)]
pub enum Link<'a> {
    Text {
        title: CowStr<'a>,
        elems: Vec<RichText<'a>>,
        dest_url: CowStr<'a>,
    },
    Footnote(CowStr<'a>),
}

#[derive(Debug, Clone)]
pub struct CodeBlock<'a> {
    pub lang: CowStr<'a>,
    pub content: CowStr<'a>,
}

#[derive(Debug, Clone)]
pub enum RichText<'a> {
    RawText(CowStr<'a>),
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
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum ListStyle {
    Unordered,
    Ordered(u64),
}

#[derive(Debug, Clone)]
pub struct ListItem<'a>(pub Vec<RichText<'a>>);

#[derive(Debug, Clone)]
pub struct List<'a> {
    pub list_style: ListStyle,
    pub elems: Vec<ListItem<'a>>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum BreakType {
    Rule,
    SoftLine,
    HardLine,
}

#[derive(Debug, Clone)]
pub struct Heading<'a> {
    pub level: HeadingLevel,
    pub text: CowStr<'a>,
    pub id: CowStr<'a>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum HeadingLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub struct RichTextOptions {
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
            Event::Text(text) => Ok(RichText::RawText(text)),
            Event::Code(code) => Ok(RichText::InlineCode(code)),
            Event::InlineMath(tex) => todo!("latex {tex}"),
            Event::DisplayMath(tex) => todo!("latex {tex}"),
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
            Event::FootnoteReference(id) => Ok(RichText::InternalLink(Link::Footnote(id))),
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
                    self.handle_html(html).map(ControlFlow::Continue)
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

        let tag_name = match &elem {
            XmlEvent::StartElement { name, .. } => {
                if let Some(end) = &end_name {
                    assert_eq!(name, end, "Invalid xhtml tag {name:?}");
                    return Some(RichText::Xhtml(InlineXhtml::Node(XmlNode::Inline(elem))));
                }
                name
            }
            e => panic!("Unexpected inline xhtml {e:?}"),
        };
        let (elems, end) = self.to_end()?;

        match end {
            EndMarker::XhtmlTag(tag) => {
                assert_eq!(tag_name.borrow(), Name::from(&*tag));
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

                let id = id.unwrap_or_else(|| {
                    let id = helpers::name_to_id(&text);

                    CowStr::Boxed(id.into_boxed_str())
                });

                let text = CowStr::Boxed(text.into_boxed_str());
                Some(RichText::Heading(Heading { level, text, id }))
            }
            Tag::BlockQuote(_) => {
                let (elems, _) = self.to_end()?;

                Some(RichText::BlockQuote(elems))
            }
            Tag::CodeBlock(code_block_kind) => {
                let lang = match code_block_kind {
                    CodeBlockKind::Fenced(lang) => lang,
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
                    title,
                    elems,
                    dest_url,
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
                    title,
                    elems,
                    dest_url,
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

#[derive(Clone, Debug)]
pub struct BookChapter<'a> {
    pub src_path: &'a Path,
    pub dest_path: &'a Path,
    pub content: Vec<RichText<'a>>,
}

impl<'a> BookChapter<'a> {
    pub fn from_chapter(ch: &'a mdbook::book::Chapter, opts: RichTextOptions) -> Option<Self> {
        let src_path = ch.source_path.as_ref()?;
        let dest_path = ch.path.as_ref()?;
        let content = RichTextParser::new(&ch.content, opts).collect();

        Some(Self {
            src_path,
            dest_path,
            content,
        })
    }
}

#[derive(Clone)]
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

#[derive(Clone, Debug)]
pub struct Book<'a> {
    pub title: &'a str,
    pub tree: NavTree<'a>,
    pub extra_files: &'a [ExtraItem],
}

impl<'a> Book<'a> {
    pub fn build<A: Borrow<BookItem>>(
        title: &'a str,
        items: &'a [A],
        opts: RichTextOptions,
        extra_files: &'a [ExtraItem],
    ) -> Book<'a> {
        Book {
            title,
            tree: NavTree::from_items(items, opts),
            extra_files,
        }
    }
}
