use std::{
    borrow::{Borrow, Cow},
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
};


use serde::de::DeserializeSeed;

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum InlineStrLen {
    _0,
    _1,
    _2,
    _3,
    _4,
    _5,
    _6,
    _7,
    _8,
    _9,
    _10,
    _11,
    _12,
    _13,
    _14,
    _15,
    _16,
    _17,
    _18,
    _19,
    _20,
    _21,
    _22,
    _23,
}

const MAX_LEGAL_LEN: usize = (InlineStrLen::_23 as usize) + 1;

const MAX_TOTAL_INLINE_STR_LEN: usize = core::mem::size_of::<&'static str>() + core::mem::align_of::<usize>();

#[derive(Copy, Clone)]
pub struct InlineStr {
    _align: [usize;0],
    len: InlineStrLen,
    body: [u8; MAX_TOTAL_INLINE_STR_LEN - 1],
}

impl InlineStr {
    pub const fn from_str(st: &str) -> Option<InlineStr> {
        let len = st.len();

        if len < const {
            if MAX_LEGAL_LEN < MAX_TOTAL_INLINE_STR_LEN {
                MAX_LEGAL_LEN
            } else {
                MAX_TOTAL_INLINE_STR_LEN
            }
        } {
            let bytes = st.as_bytes();
            let len = unsafe { core::mem::transmute(len as u8)};

            let mut str = InlineStr{_align: [], len, body: [0; _]};

            let mut i = 0;

            while i < (len as usize) {
                str.body[i] = bytes[i];
                i += 1;
            }

            Some(str)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        let len = self.len as usize;

        unsafe { core::str::from_utf8_unchecked(self.body.get_unchecked(..len))}        
    }
}

impl Deref for InlineStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

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
            Self::Borrowed(br) => match InlineStr::from_str(br) {
                Some(inl) => CowStr::Inline(inl),
                None => CowStr::Boxed(br.into()),
            },
        }
    }
}

impl<'a> From<CowStr<'a>> for String {
    fn from(value: CowStr<'a>) -> Self {
        match value {
            CowStr::Boxed(st) => st.into_string(),
            CowStr::Borrowed(st) => st.to_string(),
            CowStr::Inline(st) => st.as_str().to_string(),
        }
    }
}

impl<'a> From<CowStr<'a>> for pulldown_cmark::CowStr<'a> {
    fn from(value: CowStr<'a>) -> Self {
        match value {
            CowStr::Boxed(val) => Self::Boxed(val),
            CowStr::Borrowed(val) => Self::Borrowed(val),
            CowStr::Inline(val) => match pulldown_cmark::InlineStr::try_from(&*val) {
                Ok(inl) => Self::Inlined(inl),
                Err(_) => Self::Boxed(Box::from(val.as_str())),
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
        match InlineStr::from_str(&*value) {
            Some(val) => Self::Inline(val),
            None => Self::Boxed(value.into_boxed_str()),
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
            pulldown_cmark::CowStr::Inlined(inl) => {
                let st = &*inl;
                match InlineStr::from_str(st) {
                    Some(inl) => Self::Inline(inl),
                    None => Self::Boxed(Box::from(st))
                }
            },
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
