use mdbook::{renderer::RenderContext, BookItem};
use mdbook_fiction_tools::{
    config::SerList,
    epub::{
        config::{EpubConfig, EpubOutputType},
        write_epub,
    },
    helpers,
};
use std::{fs, io};

fn main() -> io::Result<()> {
    let mut stdin = io::stdin();
    let ctx = RenderContext::from_json(&mut stdin)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let config: EpubConfig = ctx
        .config
        .get_deserialized_opt("output.epub-fancy")
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        .unwrap_or_default();

    let dest = ctx.destination.clone();

    fs::create_dir_all(&dest)?;

    for output in config
        .output
        .unwrap_or(SerList::SingleItem(EpubOutputType::Full))
    {
        match output {
            EpubOutputType::Chapter => todo!(),
            EpubOutputType::Part => {
                for c in ctx.book.iter() {
                    match c {
                        BookItem::PartTitle(title) => {
                            let id = helpers::name_to_id(title);
                            let mut path = dest.clone();
                            path.push(id);
                            path.set_extension("epub");

                            let file = fs::File::create(path)?;

                            write_epub(file, core::iter::empty())?;
                        }
                        _ => {}
                    }
                }
            }
            EpubOutputType::Full => {
                let mut path = dest.clone();
                match &config.output_files.full {
                    Some(name) => path.push(name),
                    None => {
                        match &ctx.config.book.title {
                            Some(title) => path.push(helpers::name_to_id(title)),
                            None => path.push("book"),
                        }

                        path.set_extension("epub");
                    }
                }
                let file = fs::File::create(path)?;

                write_epub(file, core::iter::empty())?;
            }
        }
    }
    Ok(())
}
