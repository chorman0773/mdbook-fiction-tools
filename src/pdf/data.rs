use std::{
    cmp::{Eq, Ord, PartialEq, PartialOrd},
    hash::Hash,
    io,
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub mod dictionary;

pub use dictionary::Dictionary;

use super::file::PdfWriter;

#[derive(Copy, Clone, Debug, Default)]
pub struct Real(f32);

impl core::fmt::Display for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
    Name(Name),
    Null,
    String(String),
    CharString(String),
    Array(Vec<Object>),
    Dictionary(Dictionary),

    Stream(Box<StreamBody>),
    Indirect(Rc<Object>),
}

impl Object {
    pub fn write<W: io::Write>(&self, w: &mut PdfWriter<W>) -> io::Result<()> {
        use io::Write as _;
        match self {
            Object::Bool(val) => write!(w, "{val}"),
            Object::Integer(val) => write!(w, "{val}"),
            Object::Real(val) => write!(w, "{val}"),
            Object::Name(val) => write!(w, "{val}"),
            Object::Null => write!(w, "null"),
            Object::String(s) => {
                let mut last_match_end = 0;
                for (idx, m) in s.match_indices(&['\n', '\r', '(', ')', '\\']) {
                    let substr = &s[last_match_end..idx];
                    assert_eq!(m.len(), 1);
                    last_match_end = idx + 1;
                    write!(w, "{substr}")?;
                    let b = m.as_bytes()[0];
                    write!(w, "#{b:03o}")?;
                }
                let substr = &s[last_match_end..];
                write!(w, "{substr}")
            }
            Object::CharString(s) => {
                let mut last_match_end = 0;
                w.write_all(b"\xEF\xBB\xBF")?;
                for (idx, m) in s.match_indices(&['\n', '\r', '(', ')', '\\']) {
                    let substr = &s[last_match_end..idx];
                    assert_eq!(m.len(), 1);
                    last_match_end = idx + 1;
                    write!(w, "{substr}")?;
                    let b = m.as_bytes()[0];
                    write!(w, "#{b:03o}")?;
                }
                let substr = &s[last_match_end..];
                write!(w, "{substr}")
            }
            Object::Array(vec) => {
                let mut sep = "";
                write!(w, "[")?;

                for obj in vec {
                    write!(w, "{sep}")?;
                    sep = " ";
                    obj.write(w)?;
                }
                write!(w, "]")
            }
            Object::Dictionary(dictionary) => dictionary.write(w),
            Object::Stream(stream_body) => {
                stream_body.extra_attributes.write(w)?;
                write!(w, "stream")?;
                w.write_all(&stream_body.data)?;
                write!(w, "endstream")
            }
            Object::Indirect(object) => w.write_indirect_object(object),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Name(String);

impl core::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("/")?;

        let mut last_match_end = 0;

        for (idx, m) in self.match_indices(&[
            '\x00', '\x09', '\n', '\r', '\x0C', ' ', '(', ')', '<', '>', '[', ']', '{', '}', '/',
            '%', '#',
        ]) {
            let substr = &self[last_match_end..idx];
            assert_eq!(m.len(), 1);
            last_match_end = idx + 1;
            f.write_str(substr)?;
            let b = m.as_bytes()[0];
            write!(f, "#{b:02X}")?;
        }
        let substr = &self[last_match_end..];
        f.write_str(substr)
    }
}

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
