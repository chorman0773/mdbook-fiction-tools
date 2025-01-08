use std::{
    borrow::{Borrow, Cow},
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
};

pub use pulldown_cmark::InlineStr;
use serde::de::DeserializeSeed;

#[derive(Clone)]
pub enum CowStr<'a> {
    Boxed(Box<str>),
    Borrowed(&'a str),
    Inline(InlineStr),
}

impl<'a> Hash for CowStr<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl<'a> core::fmt::Display for CowStr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> core::fmt::Debug for CowStr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.as_str().escape_debug())
    }
}

impl<'a> CowStr<'a> {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Borrowed(b) => b,
            Self::Boxed(bx) => bx,
            Self::Inline(inl) => inl,
        }
    }

    pub fn into_static(self) -> CowStr<'static> {
        match self {
            Self::Boxed(bx) => CowStr::Boxed(bx),
            Self::Inline(inl) => CowStr::Inline(inl),
            Self::Borrowed(br) => match InlineStr::try_from(br) {
                Ok(inl) => CowStr::Inline(inl),
                Err(_) => CowStr::Boxed(br.into()),
            },
        }
    }
}

impl<'a> Deref for CowStr<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl<'a> AsRef<str> for CowStr<'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> Borrow<str> for CowStr<'a> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<'a, S: AsRef<str> + ?Sized> PartialEq<S> for CowStr<'a> {
    fn eq(&self, other: &S) -> bool {
        self.as_str() == other.as_ref()
    }
}

impl<'a> Eq for CowStr<'a> {}

impl<'a, S: AsRef<str> + ?Sized> PartialOrd<S> for CowStr<'a> {
    fn partial_cmp(&self, other: &S) -> Option<std::cmp::Ordering> {
        Some(self.as_str().cmp(other.as_ref()))
    }
}

impl<'a> Ord for CowStr<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl From<String> for CowStr<'_> {
    fn from(value: String) -> Self {
        match InlineStr::try_from(&*value) {
            Ok(val) => Self::Inline(val),
            Err(_) => Self::Boxed(value.into_boxed_str()),
        }
    }
}

impl<'a, S: ?Sized + AsRef<str>> From<&'a S> for CowStr<'a> {
    fn from(value: &'a S) -> Self {
        Self::Borrowed(value.as_ref())
    }
}

impl<'a> From<pulldown_cmark::CowStr<'a>> for CowStr<'a> {
    fn from(value: pulldown_cmark::CowStr<'a>) -> Self {
        match value {
            pulldown_cmark::CowStr::Boxed(bx) => Self::Boxed(bx),
            pulldown_cmark::CowStr::Borrowed(br) => Self::Borrowed(br),
            pulldown_cmark::CowStr::Inlined(inl) => Self::Inline(inl),
        }
    }
}

impl<'a> serde::Serialize for CowStr<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl<'de, 'a> serde::Deserialize<'de> for CowStr<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let not_borrowed = MaybeBorrowed::new();

        let inner = not_borrowed.deserialize(deserializer)?;

        Ok(inner.into_static())
    }
}

#[derive(Copy, Clone)]
pub struct MaybeBorrowed<'a>(PhantomData<CowStr<'a>>);

impl<'a> MaybeBorrowed<'a> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a> serde::de::DeserializeSeed<'a> for MaybeBorrowed<'a> {
    type Value = CowStr<'a>;
    fn deserialize<D>(self, deserializer: D) -> Result<CowStr<'a>, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        struct Visitor<'a>(PhantomData<CowStr<'a>>);

        impl<'a> serde::de::Visitor<'a> for Visitor<'a> {
            type Value = CowStr<'a>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(CowStr::Borrowed(v).into_static())
            }

            fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(CowStr::Borrowed(v))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v.into())
            }
        }

        deserializer.deserialize_string(Visitor(PhantomData))
    }
}
