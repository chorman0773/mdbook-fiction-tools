use bookir::{nav::NavTree, Book, BookChapter, ExtraItem, RichTextParser};
use config::{Config, OutputFile, OutputType, SerList};
use helpers::name_to_id;
use mdbook::{book::Chapter, renderer::RenderContext, BookItem};
use pulldown_cmark::{Options, Parser};

use std::{
    borrow::Cow,
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

#[cfg(feature = "xhtml")]
pub mod xhtml;

pub mod bookir;

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
    mut visitor: impl for<'a> FnMut(&Path, &Path, bookir::Book<'a>, &C, &Output) -> io::Result<()>,
    mut options: bookir::RichTextOptions,
) -> io::Result<()> {
    let config: C = ctx
        .config
        .get_deserialized_opt(format!("output.{output_name}"))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        .unwrap_or_default();

    let mut chapter_list = HashMap::new();
    let mut titles = HashMap::new();

    #[cfg(feature = "math")]
    {
        options.math = config.math_support;
    }

    let authors = ctx
        .config
        .book
        .authors
        .iter()
        .map(|r| &**r)
        .collect::<Vec<_>>();

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
        .map(|r| match r {
            Ok(src_path) => {
                let inner_path = src_path
                    .strip_prefix(&src)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
                    .to_path_buf();

                let content_type = config
                    .content_types
                    .get(&inner_path)
                    .cloned()
                    .map(Cow::Owned)
                    .unwrap_or_else(|| helpers::media_type_from_file(&inner_path));

                Ok(ExtraItem {
                    dest_path: inner_path,
                    src_path: src_path,
                    content_type,
                })
            }
            Err(e) => Err(e),
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

                            let title = title
                                .split_once("{")
                                .map_or(&**title, |(title, _)| title)
                                .trim();
                            let path = match config.output_files.individual_files.get(&id) {
                                Some(OutputFile::Path(path)) => Path::new(path),
                                None | Some(OutputFile::Enabled(true)) => Path::new(&id),
                                Some(OutputFile::Enabled(false)) => continue,
                            };

                            let part = Output::Part(id.clone());

                            let mut nav =
                                NavTree::from_items(&chapter_list[&Output::ByPartHead], options);
                            nav.append_tree(NavTree::from_items(&chapter_list[&part], options));
                            nav.append_tree(NavTree::from_items(
                                &chapter_list[&Output::ByPartTail],
                                options,
                            ));

                            let book = bookir::Book {
                                title,
                                tree: nav,
                                extra_files: &extra_files,
                                authors: &authors,
                                id: &id,
                            };

                            visitor(path, &src, book, &config, &part)?;
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

                let book = Book::build(
                    ctx.config
                        .book
                        .title
                        .as_deref()
                        .unwrap_or("Placeholder Title"),
                    &chapter_list[&Output::Full],
                    options,
                    &extra_files,
                    &authors,
                    title_id.as_deref().unwrap_or("book"),
                );

                visitor(path, &src, book, &config, &Output::Full)?;
            }
        }
    }
    Ok(())
}
