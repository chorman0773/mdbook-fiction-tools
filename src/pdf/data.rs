use std::{
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub mod dictionary;

pub use dictionary::Dictionary;

#[derive(Copy, Clone, Debug, Default)]
pub struct Real(f32);

impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Real {}

impl Ord for Real {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for Real {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Real {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u32(self.0.to_bits())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Object {
    Bool(bool),
    Integer(u32),
    Real(Real),
    String(String),
    Name(Name),
    Array(Vec<Object>),
    Dictionary(Dictionary),
    Null,
    Stream(Box<StreamBody>),
    Indirect(Rc<Object>),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Name(String);

impl Name {
    pub const fn new() -> Name {
        Name(String::new())
    }

    pub const fn from_string(st: String) -> Name {
        Name(st)
    }

    pub fn from_str<S: AsRef<str>>(st: S) -> Name {
        Name(st.as_ref().into())
    }
}

impl<S> From<S> for Name
where
    String: From<S>,
{
    fn from(value: S) -> Self {
        Self(value.into())
    }
}

impl Deref for Name {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StreamBody {
    pub extra_attributes: Dictionary,
    pub data: Vec<u8>,
}
