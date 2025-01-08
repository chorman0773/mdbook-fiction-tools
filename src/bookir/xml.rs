use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use xml::{attribute::Attribute, name::Name, namespace::Namespace, writer::XmlEvent};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct XmlElem {
    pub name: String,
    pub attrs: IndexMap<String, String>,
}

impl<'a> From<&'a XmlElem> for XmlEvent<'a> {
    fn from(value: &'a XmlElem) -> Self {
        XmlEvent::StartElement {
            name: Name::from(&*value.name),
            attributes: std::borrow::Cow::Owned(
                value
                    .attrs
                    .iter()
                    .map(|(k, v)| Attribute::new(Name::from(&**k), v))
                    .collect(),
            ),
            namespace: std::borrow::Cow::Owned(Namespace::empty()),
        }
    }
}
