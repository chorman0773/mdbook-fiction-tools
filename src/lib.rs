use config::{Config, OutputFile, OutputType, SerList};
use helpers::name_to_id;
use mdbook::{
    book::{Book, Chapter},
    renderer::RenderContext,
    BookItem,
};

use std::{
    collections::HashMap,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};

pub mod add_copyright;
pub mod config;
#[cfg(feature = "epub")]
pub mod epub;
pub mod helpers;

#[cfg(feature = "pdf")]
pub mod pdf;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum Output {
    Full,
    Part(String),
    ByPartHead,
    ByPartTail,
}

pub fn gen_collected_output<C: Config + for<'a> serde::Deserialize<'a>>(
    ctx: &RenderContext,
    output_name: impl core::fmt::Display,
    mut visitor: impl FnMut(
        &Path,
        &Path,
        &str,
        &mut dyn Iterator<Item = &'_ BookItem>,
        &C,
        &[PathBuf],
        &Output,
    ) -> io::Result<()>,
) -> io::Result<()> {
    let config: C = ctx
        .config
        .get_deserialized_opt(format!("output.{output_name}"))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        .unwrap_or_default();

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

    for output in config.outputs() {
        match output {
            OutputType::Chapter => todo!(),
            OutputType::Part => {
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
                                chapter_list.insert(Output::Part(part), chapters);
                            }
                        }
                        it @ BookItem::PartTitle(title) => {
                            has_entries = true;
                            let id = name_to_id(&title);
                            let chapters = core::mem::take(&mut cur_part_chapters);
                            if let Some(part) = cur_part.take() {
                                chapter_list.insert(Output::Part(part), chapters);
                            }
                            titles.insert(
                                Output::Part(id.clone()),
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

                if let Some(part) = cur_part.take() {
                    chapter_list.insert(Output::Part(part), cur_part_chapters);
                }
                chapter_list.insert(Output::ByPartHead, always_include_head);
                chapter_list.insert(Output::ByPartTail, always_include_tail);
            }
            OutputType::Full => {
                let mut items = Vec::new();

                for item in &ctx.book.sections {
                    match item {
                        BookItem::Separator => {}
                        item => items.push(item),
                    }
                }
                chapter_list.insert(Output::Full, items);
                titles.insert(
                    Output::Full,
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

    for output in config.outputs() {
        match output {
            OutputType::Chapter => todo!(),
            OutputType::Part => {
                for c in ctx.book.iter() {
                    match c {
                        BookItem::PartTitle(title) => {
                            let id = helpers::name_to_id(title);

                            let path = match config.output_files.individual_files.get(&id) {
                                Some(OutputFile::Path(path)) => Path::new(path),
                                None | Some(OutputFile::Enabled(true)) => Path::new(&id),
                                Some(OutputFile::Enabled(false)) => continue,
                            };

                            let part = Output::Part(id.clone());

                            let mut iter = chapter_list
                                .get(&Output::ByPartHead)
                                .into_iter()
                                .flatten()
                                .chain(&chapter_list[&part])
                                .chain(chapter_list.get(&Output::ByPartTail).into_iter().flatten())
                                .copied();

                            visitor(path, &src, title, &mut iter, &config, &extra_files, &part)?;
                        }
                        _ => {}
                    }
                }
            }
            OutputType::Full => {
                let title_id = ctx.config.book.title.as_deref().map(helpers::name_to_id);

                let path = match &config.output_files.full {
                    Some(OutputFile::Path(name)) => Path::new(name),
                    None | Some(OutputFile::Enabled(true)) => {
                        Path::new(title_id.as_deref().unwrap_or("book"))
                    }
                    Some(OutputFile::Enabled(false)) => return Ok(()),
                };

                visitor(
                    path,
                    &src,
                    &titles[&Output::Full],
                    &mut chapter_list[&Output::Full].iter().copied(),
                    &config,
                    &extra_files,
                    &Output::Full,
                )?;
            }
        }
    }
    Ok(())
}
