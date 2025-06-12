use std::{
    borrow::{Borrow, BorrowMut},
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use krilla::font::Font;

pub struct ConstFont {
    inner: UnsafeCell<ConstFontInner>,
}

impl Clone for ConstFont {
    fn clone(&self) -> Self {
        // SAFETY:
        // `ConstFont` does not permit multple threads or transient references while the value can be modified
        // Therefore it is safe to treat the inner value as a safe reference.
        let inner = unsafe { &*self.inner.get() };

        Self::from_inner(inner.clone())
    }
}

impl core::fmt::Debug for ConstFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = unsafe { &*self.inner.get() };

        f.debug_struct("ConstFont").field("inner", inner).finish()
    }
}

impl ConstFont {
    const fn from_inner(inner: ConstFontInner) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
        }
    }

    pub const fn from_font(f: Font) -> Self {
        Self::from_inner(ConstFontInner::Instantiated(f))
    }

    pub const fn from_bytes(x: &'static [u8]) -> Self {
        Self::from_inner(ConstFontInner::Blob(x))
    }

    pub fn font(&self) -> Option<&Font> {
        let ptr = self.inner.get();

        match unsafe { &*ptr } {
            ConstFontInner::Instantiated(f) => Some(f),
            ConstFontInner::Blob(b) => {
                let font = Font::new(Arc::new(*b), 0, vec![])?;

                unsafe {
                    ptr.write(ConstFontInner::Instantiated(font));
                }

                match unsafe { &*ptr } {
                    ConstFontInner::Instantiated(f) => Some(f),
                    _ => unreachable!("We just set this to Instantiated"),
                }
            }
        }
    }

    pub fn font_mut(&mut self) -> Option<&mut Font> {
        match self.inner.get_mut() {
            // SAFETY: get_or_insert_mut
            ConstFontInner::Instantiated(f) => Some(unsafe { &mut *(f as *mut _) }),
            ConstFontInner::Blob(b) => {
                let font = Font::new(Arc::new(*b), 0, vec![])?;

                *self.inner.get_mut() = ConstFontInner::Instantiated(font);

                match self.inner.get_mut() {
                    ConstFontInner::Instantiated(f) => Some(f),
                    _ => unreachable!("We just set this to Instantiated"),
                }
            }
        }
    }
}

impl Deref for ConstFont {
    type Target = Font;
    #[track_caller]
    fn deref(&self) -> &Self::Target {
        self.font()
            .expect("Deref of `ConstFont` failed to instantiate")
    }
}

impl DerefMut for ConstFont {
    #[track_caller]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.font_mut().expect("Deref of `ConstFont` failed")
    }
}

impl Borrow<Font> for ConstFont {
    #[track_caller]
    fn borrow(&self) -> &Font {
        self.font().expect("borrow of `ConstFont` failed")
    }
}

impl BorrowMut<Font> for ConstFont {
    fn borrow_mut(&mut self) -> &mut Font {
        self.font_mut().expect("borrow of `ConstFont` failed")
    }
}

impl PartialEq for ConstFont {
    fn eq(&self, other: &Self) -> bool {
        match (unsafe { &*self.inner.get() }, unsafe {
            &*other.inner.get()
        }) {
            // Don't instantiate the fonts if we're both static
            (ConstFontInner::Blob(b1), ConstFontInner::Blob(b2)) => b1 == b2,
            _ => self.font() == other.font(),
        }
    }
}

impl PartialEq<Font> for ConstFont {
    fn eq(&self, other: &Font) -> bool {
        self.font() == Some(other)
    }
}

impl PartialEq<ConstFont> for Font {
    fn eq(&self, other: &ConstFont) -> bool {
        Some(self) == other.font()
    }
}

impl Eq for ConstFont {}

impl core::hash::Hash for ConstFont {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.font() {
            Some(h) => h.hash(state),
            None => {
                // We failed to instantiate fonts fall back to hashing the bytes.
                // This is an error state for sure, but not one we should worry about
                (unsafe { &*self.inner.get() }).hash(state)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ConstFontInner {
    Blob(&'static [u8]),
    Instantiated(Font),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FontSet {
    pub base: ConstFont,
    pub italics: ConstFont,
    pub bold: ConstFont,
    pub bold_italics: ConstFont,
}

macro_rules! default_font {
    ($name:ident => $base:literal) => {
        pub const $name: FontSet = {
            #[used]
            static LICENSE: &str = include_str!(concat!($base, "-LICENSE.txt"));

            FontSet {
                base: ConstFont::from_bytes(include_bytes!(concat!($base, "-Regular.ttf"))),
                italics: ConstFont::from_bytes(include_bytes!(concat!($base, "-Italic.ttf"))),
                bold: ConstFont::from_bytes(include_bytes!(concat!($base, "-Bold.ttf"))),
                bold_italics: ConstFont::from_bytes(include_bytes!(concat!(
                    $base,
                    "-BoldItalic.ttf"
                ))),
            }
        };
    };
}

default_font!(SOURCE_CODE_PRO => "SourceCodePro");
default_font!(OPEN_SANS => "OpenSans");
