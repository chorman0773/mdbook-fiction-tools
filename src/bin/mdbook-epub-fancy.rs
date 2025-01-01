use mdbook::renderer::RenderContext;
use mdbook_fiction_tools::{
    epub::{
        config::{EpubConfig, PackageId},
        info::EpubFileInfo,
        write_epub,
    },
    gen_collected_output, helpers, Output,
};
use std::{fs, io};
use uuid::Uuid;

fn main() -> io::Result<()> {
    let mut stdin = io::stdin();
    let ctx = RenderContext::from_json(&mut stdin)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dest = ctx.destination.clone();

    fs::create_dir_all(&dest)?;

    gen_collected_output::<EpubConfig>(
        &ctx,
        "epub-fancy",
        |path, title, chapters, config, extra_files, output| {
            let path = {
                let mut dest = dest.clone();
                dest.push(path);
                dest.set_extension("epub");
                dest
            };
            let file = fs::File::create(path)?;

            let id = match output {
                Output::Full => config.file_ids.full.as_ref().cloned(),
                Output::Part(id) => config.file_ids.individual_files.get(id).cloned(),
                _ => None,
            };

            let info = EpubFileInfo {
                title: title.to_string(),
                ident: id.unwrap_or_else(|| PackageId::Uuid {
                    uuid: Uuid::now_v7(),
                }),
                lang: ctx
                    .config
                    .book
                    .language
                    .clone()
                    .unwrap_or_else(|| "en-us".to_string()),
                creators: ctx.config.book.authors.clone(),
            };

            let id = ctx
                .config
                .book
                .title
                .as_deref()
                .map(helpers::name_to_id)
                .unwrap_or_else(|| "package".to_string());

            write_epub(file, chapters, info, id, extra_files, &ctx.config.book.src)?;

            Ok(())
        },
    )
}
