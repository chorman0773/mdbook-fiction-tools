use mdbook::book::Chapter;
use zip::write::ZipWriter;

pub mod config;
pub mod info;
pub mod nav;

pub fn write_epub<'a, W: std::io::Write + std::io::Seek, I: IntoIterator<Item = &'a Chapter>>(
    writer: W,
    chapters: I,
) -> std::io::Result<()> {
    let mut file = ZipWriter::new(writer);

    file.set_comment("mdbook-output");

    file.finish()?;
    Ok(())
}
