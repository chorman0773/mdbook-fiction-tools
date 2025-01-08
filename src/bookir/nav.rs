use std::borrow::{Borrow, Cow};
use std::path::PathBuf;
use std::thread::current;

use mdbook::BookItem;
use serde::{Deserialize, Serialize};
use xml::name::Name;
use xml::namespace::Namespace;
use xml::writer::{EventWriter, XmlEvent};

use super::{BookChapter, CowStr, RichTextOptions};

#[derive(Clone, Debug, Serialize)]
pub struct NavTree<'a>(Vec<NavNode<'a>>);

impl<'a> NavTree<'a> {
    pub const fn new() -> Self {
        Self { 0: Vec::new() }
    }

    pub const fn from_vec(v: Vec<NavNode<'a>>) -> Self {
        Self { 0: v }
    }

    pub fn append_tree(&mut self, tree: NavTree<'a>) {
        self.0.extend(tree.0);
    }

    pub fn from_items<A: Borrow<BookItem>>(items: &'a [A], opts: RichTextOptions) -> Self {
        let mut node = Vec::new();
        let mut current_part = Vec::new();
        let mut part_title = None::<&str>;
        for item in items {
            match item.borrow() {
                BookItem::Chapter(ch) => {
                    let content = BookChapter::from_chapter(ch, opts);

                    let children = if ch.sub_items.is_empty() {
                        None
                    } else {
                        Some(Self::from_items(&ch.sub_items, opts))
                    };

                    let heading = match content {
                        Some(chapter) => NavHeading::Chapter(CowStr::Borrowed(&ch.name), chapter),
                        None => NavHeading::UnboundChapter(CowStr::Borrowed(&ch.name)),
                    };
                    current_part.push(NavNode { heading, children })
                }
                BookItem::PartTitle(title) => {
                    if let Some(part_title) = part_title.replace(title) {
                        node.push(NavNode {
                            heading: NavHeading::Heading(CowStr::Borrowed(part_title)),
                            children: Some(NavTree::from_vec(core::mem::take(&mut current_part))),
                        });
                    } else {
                        node.extend(core::mem::take(&mut current_part));
                    }
                }
                BookItem::Separator => {
                    if let Some(part_title) = part_title.take() {
                        node.push(NavNode {
                            heading: NavHeading::Heading(CowStr::Borrowed(part_title)),
                            children: Some(NavTree::from_vec(core::mem::take(&mut current_part))),
                        });
                    }
                }
            }
        }

        if let Some(part_title) = part_title.take() {
            node.push(NavNode {
                heading: NavHeading::Heading(CowStr::Borrowed(part_title)),
                children: Some(NavTree::from_vec(core::mem::take(&mut current_part))),
            });
        } else {
            node.extend(core::mem::take(&mut current_part));
        }

        Self::from_vec(node)
    }

    pub fn push(&mut self, node: NavNode<'a>) {
        self.0.push(node);
    }

    pub fn nested(&self) -> Nested<'_, 'a> {
        Nested {
            stack: vec![self.0.iter()],
        }
    }

    pub fn iter(&self) -> Iter<'_, 'a> {
        Iter(self.0.iter())
    }
}

impl<'tree, 'src> IntoIterator for &'tree NavTree<'src> {
    type IntoIter = Iter<'tree, 'src>;
    type Item = &'tree NavNode<'src>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter<'tree, 'src>(core::slice::Iter<'tree, NavNode<'src>>);

impl<'tree, 'src> Iterator for Iter<'tree, 'src> {
    type Item = &'tree NavNode<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct Nested<'tree, 'src> {
    stack: Vec<core::slice::Iter<'tree, NavNode<'src>>>,
}

impl<'tree, 'src> Iterator for Nested<'tree, 'src> {
    type Item = &'tree NavNode<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let head = self.stack.last_mut()?;
            if let Some(item) = head.next() {
                if let Some(children) = &item.children {
                    self.stack.push(children.0.iter());
                }
                break Some(item);
            } else {
                self.stack.pop();
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct NavNode<'a> {
    pub heading: NavHeading<'a>,
    pub children: Option<NavTree<'a>>,
}

#[derive(Clone, Debug, Serialize)]
pub enum NavHeading<'a> {
    Chapter(CowStr<'a>, BookChapter<'a>),
    UnboundChapter(CowStr<'a>),
    Heading(CowStr<'a>),
}
