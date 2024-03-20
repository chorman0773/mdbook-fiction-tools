use std::collections::HashMap;
use std::path::Path;
use std::{collections::HashSet, path::PathBuf};

use mdbook::book::Chapter;
use mdbook::errors::{Error as MdError, Result as MdResult};
use mdbook::preprocess::Preprocessor;

use mdbook::BookItem;
use serde_derive::Deserialize;

use pulldown_cmark::{CowStr, Event, Options as MdParseOptions, Parser, Tag};
use pulldown_cmark_to_cmark::{cmark_resume_with_options, Options as MdPrintOptions};

#[derive(Deserialize)]
pub struct FileSet {
    #[serde(default)]
    pub include: Option<HashSet<PathBuf>>,
    #[serde(default)]
    pub exclude: HashSet<PathBuf>,
}

impl FileSet {
    pub fn contains_file(&self, p: &Path) -> bool {
        if let Some(include) = &self.include {
            if !include.contains(p) {
                return false;
            }
        }

        !self.exclude.contains(p)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct AddCopyrightPreprocessorConfig {
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default)]
    pub renderers: Vec<String>,
    #[serde(default)]
    pub copyright_stub: Option<PathBuf>,
    #[serde(flatten)]
    pub base_set: FileSet,
    #[serde(flatten)]
    pub renderer_sets: HashMap<String, FileSet>,
}

pub struct AddCopyrightPreprocessor {}

impl Preprocessor for AddCopyrightPreprocessor {
    fn name(&self) -> &str {
        "add-copyright"
    }
    fn supports_renderer(&self, _renderer: &str) -> bool {
        true
    }

    fn run(
        &self,
        ctx: &mdbook::preprocess::PreprocessorContext,
        mut book: mdbook::book::Book,
    ) -> MdResult<mdbook::book::Book> {
        let root = &ctx.config.book.src;

        let config = ctx
            .config
            .get_preprocessor("add-copyright")
            .ok_or_else(|| {
                MdError::msg("Could not find configuration for add-copyright preprocessor")
            })?;

        let renderer = &ctx.renderer;

        let st = toml::to_string(config).map_err(MdError::new)?;

        let config: AddCopyrightPreprocessorConfig = toml::from_str(&st).map_err(MdError::new)?;

        let mut stub_path = root.clone();
        stub_path.push(
            config
                .copyright_stub
                .as_deref()
                .unwrap_or_else(|| Path::new("COPYRIGHT-STUB.md")),
        );

        let stub = std::fs::read_to_string(stub_path).map_err(MdError::new)?;

        try_for_each_mut(
            &mut |item| {
                match item {
                    BookItem::Chapter(ch) => {
                        Self::update_chapter(ch, &config, renderer, root, &stub)?;
                    }
                    _ => {}
                }
                Ok(())
            },
            &mut book.sections,
        )?;

        Ok(book)
    }
}

pub fn try_for_each_mut<
    'a,
    F: FnMut(&mut BookItem) -> MdResult<()>,
    I: IntoIterator<Item = &'a mut BookItem>,
>(
    func: &mut F,
    it: I,
) -> MdResult<()> {
    for item in it {
        match item {
            BookItem::Chapter(ch) => {
                try_for_each_mut(func, &mut ch.sub_items)?;
            }
            _ => {}
        }

        func(item)?;
    }
    Ok(())
}

impl AddCopyrightPreprocessor {
    pub fn update_chapter(
        ch: &mut Chapter,
        config: &AddCopyrightPreprocessorConfig,
        renderer: &str,
        root: &Path,
        stub: &str,
    ) -> MdResult<()> {
        let Some(path) = &ch.path else { return Ok(()) };

        let renderer_file_set = config.renderer_sets.get(renderer);

        let is_included = config.base_set.contains_file(path)
            && renderer_file_set
                .map(|f| f.contains_file(path))
                .unwrap_or(true);

        let mut file_path = root.to_path_buf();
        file_path.push(path);

        let parse_options = MdParseOptions::ENABLE_FOOTNOTES
            | MdParseOptions::ENABLE_HEADING_ATTRIBUTES
            | MdParseOptions::ENABLE_TABLES
            | MdParseOptions::ENABLE_STRIKETHROUGH
            | MdParseOptions::ENABLE_TASKLISTS;

        let parser = Parser::new_ext(&ch.content, parse_options);

        let print_options = MdPrintOptions::default();

        let mut state = None;

        let mut output = String::new();

        for event in parser {
            match event {
                Event::Text(text) => {
                    if let Some((a, b)) = text.split_once("!{#copyright}") {
                        state = Some(
                            cmark_resume_with_options(
                                [Event::Text(CowStr::Borrowed(a))].into_iter(),
                                &mut output,
                                state.take(),
                                print_options.clone(),
                            )
                            .map_err(MdError::new)?,
                        );

                        if is_included {
                            let stub_parser = Parser::new_ext(stub, parse_options);

                            let events = stub_parser.into_iter().map(|mut m| {
                                match &mut m {
                                    Event::Start(tag) | Event::End(tag) => match tag {
                                        Tag::Link(_, dest_url, _) | Tag::Image(_, dest_url, _) => {
                                            if !dest_url.is_empty()
                                                && !dest_url.contains("://")
                                                && !dest_url.starts_with("/")
                                            {
                                                let (dotdotgroups, base) = file_path
                                                    .parent()
                                                    .unwrap()
                                                    .ancestors()
                                                    .enumerate()
                                                    .find(|(_, p)| root.starts_with(p))
                                                    .unwrap();

                                                let mut real_path = PathBuf::new();

                                                for _ in 0..dotdotgroups {
                                                    real_path.push("..");
                                                }
                                                real_path.push(root.strip_prefix(base).unwrap());
                                                real_path.push(&**dest_url);

                                                *dest_url = CowStr::Boxed(
                                                    real_path
                                                        .into_os_string()
                                                        .into_string()
                                                        .unwrap()
                                                        .into_boxed_str(),
                                                );
                                            }
                                        }
                                        _ => {}
                                    },

                                    _ => {}
                                }
                                m
                            });

                            state = Some(
                                cmark_resume_with_options(
                                    events,
                                    &mut output,
                                    state.take(),
                                    print_options.clone(),
                                )
                                .map_err(MdError::new)?,
                            )
                        }
                        state = Some(
                            cmark_resume_with_options(
                                core::iter::once(Event::Text(CowStr::Borrowed(b))),
                                &mut output,
                                state.take(),
                                print_options.clone(),
                            )
                            .map_err(MdError::new)?,
                        );
                    } else {
                        state = Some(
                            cmark_resume_with_options(
                                core::iter::once(Event::Text(text)),
                                &mut output,
                                state.take(),
                                print_options.clone(),
                            )
                            .map_err(MdError::new)?,
                        );
                    }
                }
                event => {
                    state = Some(
                        cmark_resume_with_options(
                            core::iter::once(event),
                            &mut output,
                            state.take(),
                            print_options.clone(),
                        )
                        .map_err(MdError::new)?,
                    )
                }
            }
        }

        ch.content = output;

        Ok(())
    }
}
