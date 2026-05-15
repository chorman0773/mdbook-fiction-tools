use mdbook_renderer::RenderContext;
use mdbook_fiction_tools::{
    bookir::RichTextOptions, config::BasicConfig, renderer::gen_collected_output, helpers, Output,
};
use serde_json::Error;
use std::{fs, io};
use uuid::Uuid;

fn main() -> io::Result<()> {
    let mut stdin = io::stdin();
    let ctx = RenderContext::from_json(&mut stdin)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dest = ctx.destination.clone();

    fs::create_dir_all(&dest)?;

    gen_collected_output::<BasicConfig>(
        &ctx,
        "bookir",
        |path, src, book, config, output| {
            use std::io::Write;
            let path = {
                let mut dest = dest.clone();
                dest.push(path);
                dest.set_extension("bookir");
                dest
            };
            let file = fs::File::create(path)?;

            serde_json::to_writer_pretty(file, &book).map_err(|e| {
                if let Some(kind) = e.io_error_kind() {
                    io::Error::new(kind, e)
                } else {
                    io::Error::new(io::ErrorKind::Other, e)
                }
            })
        },
        RichTextOptions {
            ..Default::default()
        },
    )
}
