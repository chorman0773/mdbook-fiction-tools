use mdbook::{renderer::RenderContext, BookItem};
use mdbook_fiction_tools::{
    config::SerList,
    epub::{
        config::{EpubConfig, EpubPackageId},
        info::EpubFileInfo,
        write_epub,
    },
    gen_collected_output,
    helpers::{self, name_to_id},
};
use std::{collections::HashMap, ffi::OsStr, fs, io};
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
        |path, title, chapters, config, extra_files| {
            let path = {
                let mut dest = dest.clone();
                dest.push(path);
                dest.set_extension("epub");
                dest
            };
            let file = fs::File::create(path)?;

            let info = EpubFileInfo {
                title: title.to_string(),
                ident: config
                    .file_ids
                    .full
                    .clone()
                    .unwrap_or_else(|| EpubPackageId::Uuid {
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
