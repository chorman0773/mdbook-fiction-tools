use std::io;

use krilla::font::Font;
use uuid::Uuid;

use crate::bookir::Book;

pub mod config;

pub fn write_pdf<W: std::io::Write>(
    file: W,
    book: Book,
    file_id: Uuid,
    def_font: Font,
    mono_font: Font,
) -> io::Result<()> {
    let mut pdf = krilla::Document::new();
    todo!()
}
