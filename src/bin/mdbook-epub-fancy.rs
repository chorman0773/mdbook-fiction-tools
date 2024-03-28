use mdbook::{renderer::RenderContext, BookItem};
use mdbook_fiction_tools::{
    config::SerList,
    epub::{
        config::{EpubConfig, EpubOutputType, EpubPackageId},
        info::EpubFileInfo,
        write_epub,
    },
    helpers::{self, name_to_id},
};
use std::{collections::HashMap, ffi::OsStr, fs, io};
use uuid::Uuid;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum EpubOutput {
    Full,
    Part(String),
    ByPartHead,
    ByPartTail,
}

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

    let mut chapter_list = HashMap::new();
    let mut titles = HashMap::new();

    let mut src = ctx.root.clone();
    src.push(&ctx.config.book.src);

    let mut extra_files = helpers::read_dir_recursive(&src)?
        .map(|e| e.map(|e| e.path()))
        .filter(|e| {
            if let Ok(path) = e {
                path.extension() != Some(OsStr::new("md"))
            } else {
                true
            }
        })
        .collect::<io::Result<Vec<_>>>()?;

    for output in config
        .output
        .as_ref()
        .unwrap_or(&SerList::SingleItem(EpubOutputType::Full))
    {
        match output {
            EpubOutputType::Chapter => todo!(),
            EpubOutputType::Part => {
                let mut always_include_head = Vec::new();
                let mut always_include_tail = Vec::new();
                let mut cur_part = None;
                let mut cur_part_chapters = Vec::new();
                let mut has_entries = false;

                for c in &ctx.book.sections {
                    match c {
                        BookItem::Separator => {
                            let chapters = core::mem::take(&mut cur_part_chapters);
                            if let Some(part) = cur_part.take() {
                                chapter_list.insert(EpubOutput::Part(part), chapters);
                            }
                        }
                        it @ BookItem::PartTitle(title) => {
                            has_entries = true;
                            let id = name_to_id(&title);
                            let chapters = core::mem::take(&mut cur_part_chapters);
                            if let Some(part) = cur_part.take() {
                                chapter_list.insert(EpubOutput::Part(part), chapters);
                            }
                            titles.insert(
                                EpubOutput::Part(id.clone()),
                                title
                                    .split_once('{')
                                    .map_or(&**title, |(l, r)| l)
                                    .to_string(),
                            );
                            cur_part = Some(id);
                        }
                        it @ BookItem::Chapter(c) => {
                            if let Some(path) = &c.path {
                                if config
                                    .always_include
                                    .contains(path.strip_prefix("..").unwrap_or(path))
                                {
                                    if has_entries {
                                        always_include_tail.push(it);
                                    } else {
                                        always_include_head.push(it);
                                    }
                                }
                            }
                            cur_part_chapters.push(it);
                        }
                    }
                }

                chapter_list.insert(EpubOutput::ByPartHead, always_include_head);
                chapter_list.insert(EpubOutput::ByPartTail, always_include_tail);
            }
            EpubOutputType::Full => {
                let mut items = Vec::new();

                for item in &ctx.book.sections {
                    match item {
                        BookItem::Separator => {}
                        item => items.push(item),
                    }
                }
                chapter_list.insert(EpubOutput::Full, items);
                titles.insert(
                    EpubOutput::Full,
                    ctx.config
                        .book
                        .title
                        .clone()
                        .map(|mut s| {
                            if let Some(idx) = s.find('{') {
                                s.truncate(idx);
                            }
                            s
                        })
                        .unwrap_or_else(|| "Epub Book".to_string()),
                );
            }
        }
    }

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
                            if let Some(file) = config.output_files.individual_files.get(&id) {
                                path.push(file);
                            } else {
                                path.push(&id);
                                path.set_extension("epub");
                            }

                            let file = fs::File::create(path)?;

                            let part_id = config.file_ids.individual_files.get(&id);

                            let output = EpubOutput::Part(id.clone());

                            let chapters = chapter_list[&EpubOutput::ByPartHead]
                                .iter()
                                .chain(&chapter_list[&output])
                                .chain(&chapter_list[&EpubOutput::ByPartTail])
                                .copied();

                            let info = EpubFileInfo {
                                title: titles.remove(&output).unwrap(),
                                ident: part_id.cloned().unwrap_or_else(|| EpubPackageId::Uuid {
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

                            write_epub(file, chapters, info, id, &extra_files, &src)?;
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

                let info =
                    EpubFileInfo {
                        title: titles.remove(&EpubOutput::Full).unwrap(),
                        ident: config.file_ids.full.clone().unwrap_or_else(|| {
                            EpubPackageId::Uuid {
                                uuid: Uuid::now_v7(),
                            }
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

                write_epub(
                    file,
                    chapter_list[&EpubOutput::Full].iter().copied(),
                    info,
                    id,
                    &extra_files,
                    &src,
                )?;
            }
        }
    }
    Ok(())
}
