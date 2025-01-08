use mdbook::renderer::RenderContext;
use mdbook_fiction_tools::{
    bookir::RichTextOptions,
    gen_collected_output, helpers,
    pdf::{config::PdfConfig, write_pdf},
    Output,
};
use std::{fs, io};
use uuid::Uuid;

fn main() -> io::Result<()> {
    let mut stdin = io::stdin();
    let ctx = RenderContext::from_json(&mut stdin)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dest = ctx.destination.clone();

    fs::create_dir_all(&dest)?;

    gen_collected_output::<PdfConfig>(
        &ctx,
        "epub-fancy",
        |path, _, book, config, output| {
            let path = {
                let mut dest = dest.clone();
                dest.push(path);
                dest.set_extension("pdf");
                dest
            };
            let file = fs::File::create(path)?;

            let file_id = match output {
                Output::Full => config.file_ids.full.as_ref().cloned(),
                Output::Part(id) => config.file_ids.individual_files.get(id).cloned(),
                _ => None,
            };

            let file_id = file_id.unwrap_or_else(|| Uuid::now_v7());
            todo!()
        },
        RichTextOptions {
            ..Default::default()
        },
    )
}
