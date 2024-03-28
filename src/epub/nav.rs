use std::borrow::Cow;
use std::path::PathBuf;

use xml::name::Name;
use xml::namespace::Namespace;
use xml::writer::{EventWriter, XmlEvent};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct NavTree {
    subnodes: Vec<NavNode>,
}

impl NavTree {
    pub const fn new() -> Self {
        Self {
            subnodes: Vec::new(),
        }
    }
    pub fn push(&mut self, node: NavNode) {
        self.subnodes.push(node);
    }
    pub fn treeify(&mut self) {
        let untreed = core::mem::take(&mut self.subnodes);
        let mut stack = vec![self];
        for node in untreed {
            // These are unsafe crimes.
            // SAFETY:
            // We don't actually mutate any previous NavTree on the stack until the previous elements are popped
            // So this lifetime extension, while sussy, isn't unsound
            let last = unsafe { core::ptr::read(stack.last_mut().unwrap()) };

            last.subnodes.push(node);

            let entry = last.subnodes.last_mut().unwrap();

            match &entry.heading {
                NavHeading::Chapter(_, _) | NavHeading::Heading(_) => {
                    stack.push(entry.children.get_or_insert_with(Self::new));
                }
                NavHeading::End => {
                    stack.pop();
                }
            }
        }

        if stack.len() != 1 {
            panic!("Unmatched begin/end groups");
        }
    }

    pub fn write_ol<W: std::io::Write>(&self, w: &mut EventWriter<W>) -> xml::writer::Result<()> {
        if self.subnodes.len() < 2 {
            // 1 element is the `End` element after tree expansion
            return Ok(());
        }
        w.write(XmlEvent::start_element("ol"))?;

        for node in &self.subnodes {
            match &node.heading {
                NavHeading::Chapter(title, path) => {
                    let path = path.to_string_lossy();

                    w.write(XmlEvent::start_element("li"))?;

                    w.write(XmlEvent::start_element("a").attr("href", &path))?;
                    w.write(XmlEvent::characters(title))?;
                    w.write(XmlEvent::end_element())?;
                }
                NavHeading::End => continue,
                NavHeading::Heading(head) => {
                    w.write(XmlEvent::start_element("li"))?;
                    w.write(XmlEvent::start_element("span"))?;
                    w.write(XmlEvent::characters(head))?;
                    w.write(XmlEvent::end_element())?;
                }
            }
            if let Some(children) = &node.children {
                children.write_ol(w)?;
            }
            w.write(XmlEvent::end_element())?;
        }
        w.write(XmlEvent::end_element())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct NavNode {
    pub heading: NavHeading,
    pub children: Option<NavTree>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum NavHeading {
    Chapter(String, PathBuf),
    Heading(String),
    End,
}
