use std::convert::Infallible;

use crate::{bookir::{self, BookChapter, RichText, RichTextOptions, XmlNode}, xhtml};

use mdbook_core::book::Chapter;
use pulldown_cmark::{Event, Tag, TagEnd};
use pulldown_cmark_to_cmark::{Error, Options, State, cmark_resume_with_options};
use xml::{EmitterConfig, EventWriter};

pub fn write_as_markdown<'a>(fmt: &mut String, rich: &RichText<'a>, state: Option<State<'a>>, options: Options<'a>) -> Result<State<'a>, Error> {
    struct StateBundle<'a, 'b> {
        state: Option<State<'a>>,
        options: Options<'a>,
        fmt: &'b mut String,
    }

    impl<'a, 'b> StateBundle<'a, 'b>{
        fn write<E: IntoIterator<Item = Event<'a>>>(&mut self, elems: E) -> Result<(), Error> {
            let state = cmark_resume_with_options(elems.into_iter(), &mut self.fmt, self.state.take(), self.options.clone())?;
            self.state = Some(state);

            Ok(())
        }

        fn write_nested(&mut self, rich: &RichText<'a>) -> Result<(), Error> {
            let state = write_as_markdown(&mut *self.fmt, rich, self.state.take(), self.options.clone())?;
            self.state = Some(state);

            Ok(())
        }
    }

    let mut state = StateBundle {state, options, fmt};

    match rich {
        RichText::RawText(cow_str) => state.write([Event::Text(cow_str.clone().into())])?,
        RichText::Comment(_) => state.write([])?,
        RichText::Xhtml(inline_xhtml) => {
            let mut buf = Vec::<u8>::new();
            xhtml::write_inline_node(inline_xhtml, &mut EventWriter::new_with_config(&mut buf, EmitterConfig::new().write_document_declaration(false))).unwrap();
            let st = String::from_utf8(buf).unwrap();

            state.write([Event::Html(pulldown_cmark::CowStr::Boxed(st.into_boxed_str()))])?;

        },
        RichText::Stylised(attributes, rich_texts) => {
            let mut start_tags = Vec::new();
            let mut end_tags = Vec::new();

            if attributes.bold {
                start_tags.push(Event::Start(Tag::Strong));
                end_tags.push(Event::End(TagEnd::Strong));
            }

            if attributes.italics {
                start_tags.push(Event::Start(Tag::Emphasis));
                end_tags.push(Event::End(TagEnd::Emphasis));
            }

            if attributes.strikethrough {
                start_tags.push(Event::Start(Tag::Strikethrough));
                end_tags.push(Event::End(TagEnd::Strikethrough));
            }
            
            state.write(start_tags)?;

            for text in rich_texts {
                state.write_nested(text)?;
            }

            state.write(end_tags.into_iter().rev())?;

        },
        RichText::Paragraph(rich_texts) => {
            state.write([Event::Start(Tag::Paragraph)])?;
            for text in rich_texts {
                state.write_nested(text)?;
            }
            state.write([Event::End(TagEnd::Paragraph)])?;
        },
        RichText::InlineCode(cow_str) => {
            state.write([Event::Code(cow_str.clone().into())])?;
        },
        RichText::CodeBlock(code) => {
            state.write([Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(code.lang.clone().into()))), Event::Text(code.content.clone().into()), Event::End(TagEnd::CodeBlock)])?;
        },
        RichText::BlockQuote(rich_texts) => {
            state.write([Event::Start(Tag::BlockQuote(None))])?;
            for text in rich_texts {
                state.write_nested(rich)?;
            }
            state.write([Event::End(TagEnd::BlockQuote(None))])?;
        },
        RichText::InternalLink(link) |
        RichText::ExternalLink(link) => {
            match link {
                bookir::Link::Text { title, elems, dest_url } => {
                    state.write([Event::Start(Tag::Link { link_type: pulldown_cmark::LinkType::Inline, dest_url: dest_url.clone().into(), title: title.clone().into(), id: pulldown_cmark::CowStr::Borrowed("") })])?;
                    for elem in elems {
                        state.write_nested(elem)?;
                    }
                    state.write([Event::End(TagEnd::Link)])?;
                },
                bookir::Link::Footnote(cow_str) => {
                    state.write([
                        Event::FootnoteReference(cow_str.clone().into())
                    ])?;
                },
            }
        },
        RichText::InternalImage(link) |
        RichText::ExternalImage(link) => {
            match link {
                bookir::Link::Text { title, elems, dest_url } => {
                    state.write([Event::Start(Tag::Image { link_type: pulldown_cmark::LinkType::Inline, dest_url: dest_url.clone().into(), title: title.clone().into(), id: pulldown_cmark::CowStr::Borrowed("") })])?;
                    for elem in elems {
                        state.write_nested(elem)?;
                    }
                    state.write([Event::End(TagEnd::Image)])?;
                },
                bookir::Link::Footnote(cow_str) => panic!("Image Footnotes not allowed"),
            }
        },
        RichText::Heading(heading) => {
            let level = match heading.level {
                bookir::HeadingLevel::H1 => pulldown_cmark::HeadingLevel::H1,
                bookir::HeadingLevel::H2 => pulldown_cmark::HeadingLevel::H2,
                bookir::HeadingLevel::H3 => pulldown_cmark::HeadingLevel::H3,
                bookir::HeadingLevel::H4 => pulldown_cmark::HeadingLevel::H4,
                bookir::HeadingLevel::H5 => pulldown_cmark::HeadingLevel::H5,
                bookir::HeadingLevel::H6 => pulldown_cmark::HeadingLevel::H6,
            };
            let events = [
                Event::Start(Tag::Heading { level, id: Some(heading.id.clone().into()), classes: Vec::new(), attrs: Vec::new() }),
                Event::Text(heading.text.clone().into()),
                Event::End(TagEnd::Heading (level)),
            ];

            state.write(events)?;
        },
        RichText::TextBreak(break_type) => {
            match break_type {
                bookir::BreakType::Rule => state.write([Event::Rule])?,
                bookir::BreakType::SoftLine => state.write([Event::SoftBreak])?,
                bookir::BreakType::HardLine => state.write([Event::HardBreak])?,
            }
        },
        RichText::List(list) => {
            let is_ordered = match list.list_style {
                bookir::ListStyle::Unordered => {state.write([Event::Start(Tag::List(None))])?; false},
                bookir::ListStyle::Ordered(start) => {state.write([Event::Start(Tag::List(Some(start)))])?; true},
            };

            for item in &list.elems {
                state.write([Event::Start(Tag::Item)])?;
                for text in &item.0 {
                    state.write_nested(text)?;
                }
                state.write([Event::End(TagEnd::Item)])?;
            }

            state.write([Event::End(TagEnd::List(is_ordered))])?;
        },
        RichText::Table(table) => {
            state.write([Event::Start(Tag::Table(table.align.iter().map(|a| match a {
                bookir::Alignment::None => pulldown_cmark::Alignment::None,
                bookir::Alignment::Left => pulldown_cmark::Alignment::Left,
                bookir::Alignment::Center => pulldown_cmark::Alignment::Center,
                bookir::Alignment::Right => pulldown_cmark::Alignment::Right,
            }).collect()))])?;

            if let Some(head) = &table.head {
                state.write([Event::Start(Tag::TableHead)])?;

                for cell in &head.elems {
                    state.write([Event::Start(Tag::TableCell)])?;
                    state.write_nested(cell)?;
                    state.write([Event::End(TagEnd::TableCell)])?;
                }

                state.write([Event::End(TagEnd::TableHead)])?;
            }

            for row in &table.body {
                state.write([Event::Start(Tag::TableRow)])?;

                for cell in &row.elems {
                    state.write([Event::Start(Tag::TableCell)])?;
                    state.write_nested(cell)?;
                    state.write([Event::End(TagEnd::TableCell)])?;
                }

                state.write([Event::End(TagEnd::TableRow)])?;
            }
            state.write([Event::End(TagEnd::Table)])?;
        },
        RichText::Macro(cow_str) => {
            let macro_text = format!("!{{{cow_str}}}");
            state.write([Event::SoftBreak, Event::Text(pulldown_cmark::CowStr::from(macro_text)), Event::HardBreak])?;
        },
    }


    Ok(state.state.unwrap())
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum VisitResult<T> {
    /// Indicates that sibling elements and subelements are not of interest to the visitor. This allows early breaking from, say, visiting a table
    IgnoreAll,
    /// Indicates that no further visitation is necessary - subelements are not of interest to the visitor
    Ignored,
    /// Indicates that subelements (if any) should be visited, but the element was not modified
    Interested,
    /// Indicates that the node should be updated to the output
    Update{
        /// The new value to insert
        result: T,
        /// Whether or not the visitor should reapplied to the result
        recurse: bool,
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum GroupKind {
    ListItem,
    TableHead,
    TableRow,
}

pub trait Visitor {
    fn visit_node<'a>(&mut self, node: &RichText<'a>) -> VisitResult<RichText<'a>>;

    fn visit_group_item(&mut self, group_kind: GroupKind) -> VisitResult<Infallible> {
        VisitResult::Interested
    }
    fn visit_end_group(&mut self, group_kind: GroupKind) {}

    fn visit_end(&mut self, node: &RichText) {}
}

impl<V: Visitor> Visitor for &mut V {
    fn visit_group_item(&mut self, group_kind: GroupKind) -> VisitResult<Infallible> {
        V::visit_group_item(self, group_kind)
    }

    fn visit_end_group(&mut self, group_kind: GroupKind) {
        V::visit_end_group(self, group_kind);
    }

    fn visit_end(&mut self, node: &RichText) {
        V::visit_end(self, node);
    }
    
    fn visit_node<'a>(&mut self, node: &RichText<'a>) -> VisitResult<RichText<'a>> {
        V::visit_node(self, node)
    }
}

impl<V: Visitor> Visitor for Box<V> {
    fn visit_group_item(&mut self, group_kind: GroupKind) -> VisitResult<Infallible> {
        V::visit_group_item(self, group_kind)
    }

    fn visit_end_group(&mut self, group_kind: GroupKind) {
        V::visit_end_group(self, group_kind);
    }

    fn visit_end(&mut self, node: &RichText) {
        V::visit_end(self, node);
    }
    
    fn visit_node<'a>(&mut self, node: &RichText<'a>) -> VisitResult<RichText<'a>> {
        V::visit_node(self, node)
    }
}

fn visit_list<'a: 'b, 'b, I: IntoIterator<Item=&'b mut RichText<'a>>>(inner: I, visitor: &mut impl Visitor) -> bool {
    let mut dirty = false;
    for inner in inner {
        let (done_sib, dirty_inner) = inner.visit_inner(visitor);

        if done_sib {
            break
        }

        dirty |= dirty_inner;
    }

    dirty
}

impl<'a> RichText<'a> {
    fn visit_inner<V: Visitor>(&mut self, visitor: &mut V) -> (bool, bool) {
        let mut dirty = match visitor.visit_node(self) {
            VisitResult::IgnoreAll => {
                return (true, false)
            },
            VisitResult::Ignored => {
                return (false, false)
            },
            VisitResult::Interested => {
                false
            },
            VisitResult::Update { result, recurse } => {
                *self = result;

                if !recurse {
                    return (false, true)
                }
                true
            },
        };

        match self {
            RichText::RawText(cow_str) => {},
            RichText::Comment(cow_str) => {},
            RichText::Xhtml(inline_xhtml) => {
                match inline_xhtml {
                    bookir::InlineXhtml::Node(XmlNode::Block(_, inner)) => {
                        dirty |= visit_list(inner, visitor);
                    },
                    bookir::InlineXhtml::Node(XmlNode::Inline(_)) |
                    bookir::InlineXhtml::Comment(_) |
                    &mut bookir::InlineXhtml::CData(_) |
                    bookir::InlineXhtml::Text(_) => {},
                }
            },
            RichText::Stylised(_, inner) => {
                dirty |= visit_list(inner, visitor);
            },
            RichText::Paragraph(inner) => {
                dirty |= visit_list(inner, visitor);
            },
            RichText::InlineCode(_) => {},
            RichText::CodeBlock(_) => {},
            RichText::BlockQuote(inner) => {
                dirty |= visit_list(inner, visitor);
            },
            RichText::InternalLink(link) |
            RichText::ExternalLink(link) |
            RichText::InternalImage(link) |
            RichText::ExternalImage(link) => {
                match link {
                    bookir::Link::Text { elems, .. } => {
                        dirty |= visit_list(elems, visitor);
                    },
                    bookir::Link::Footnote(_) => {},
                }
            },
            RichText::Heading(_) => {},
            RichText::TextBreak(_) => {},
            RichText::List(list) => {

                for item in &mut list.elems {
                    match visitor.visit_group_item(GroupKind::ListItem) {
                        VisitResult::IgnoreAll => {
                            break
                        },
                        VisitResult::Ignored => {
                            continue
                        },
                        VisitResult::Interested => {},
                        VisitResult::Update { result, .. } => match result {},
                    }
                    dirty |= visit_list(&mut item.0, visitor);
                    visitor.visit_end_group(GroupKind::ListItem);
                }

            },
            RichText::Table(table) => {
                let continue_table = if let Some(head) = &mut table.head {
                    let continue_table = match visitor.visit_group_item(GroupKind::TableHead) {
                        VisitResult::IgnoreAll => false,
                        VisitResult::Ignored => true,
                        VisitResult::Interested => {
                            dirty |= visit_list(&mut head.elems, visitor);
                            visitor.visit_end_group(GroupKind::TableHead);
                            true
                        },
                        VisitResult::Update { result, .. } => match result {},
                    };

                    continue_table
                } else {
                    true
                };

                if continue_table {
                    for row in &mut table.body {
                        match visitor.visit_group_item(GroupKind::TableRow) {
                            VisitResult::IgnoreAll => break,
                            VisitResult::Ignored => continue,
                            VisitResult::Interested => {
                                dirty |= visit_list(&mut row.elems, visitor);
                                visitor.visit_end_group(GroupKind::TableRow);
                            },
                            VisitResult::Update { result, .. } => match result {},
                        }
                    }
                }
            },
            RichText::Macro(cow_str) => {},
        }

        visitor.visit_end(self);

        (false, dirty)
    }


    pub fn visit<V: Visitor>(&mut self, visitor: &mut V) -> bool {
        let (_, dirty) = self.visit_inner(visitor);

        dirty
    }
}

pub fn preprocess_chapter<V: Visitor>(chapter: &mut BookChapter, mut visitor: V) -> bool {
    visit_list(&mut chapter.content, &mut visitor)
}

pub fn preprocess_mdbook_chapter<V: Visitor>(chapter: &mut Chapter, opts: RichTextOptions, visitor: V) -> Result<(), Error> {
    let options = pulldown_cmark_to_cmark::Options::default();
    match BookChapter::from_chapter(chapter, opts) {
        Some(mut ir_chapter) => {
            if preprocess_chapter(&mut ir_chapter, visitor) {
                let mut state = None;
                let mut nstr = String::new();

                for elem in &ir_chapter.content {
                    state = Some(write_as_markdown(&mut nstr, elem, state, options.clone())?);
                }

                chapter.content = nstr;
            }
        }
        None => {}
    }
    Ok(())
}