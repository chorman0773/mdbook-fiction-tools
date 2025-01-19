use krilla::font::Font;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct FontSet {
    pub base: Font,
    pub italics: Font,
    pub bold: Font,
    pub bold_italics: Font,
}
