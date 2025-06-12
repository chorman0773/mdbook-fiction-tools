use std::io;

use fonts::FontSet;
use krilla::metadata::Metadata;
use uuid::Uuid;

use crate::bookir::Book;

pub mod config;

pub fn write_pdf<W: std::io::Write>(
    file: W,
    book: Book,
    file_id: Uuid,
    def_font: FontSet,
    mono_font: FontSet,
) -> io::Result<()> {
    let mut pdf = krilla::Document::new();
    pdf.set_metadata(Metadata::new());
    todo!()
}

mod fonts;
