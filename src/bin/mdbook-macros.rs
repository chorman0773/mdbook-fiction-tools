use std::{collections::{HashMap, HashSet}, path::{Path, PathBuf}};

use mdbook_fiction_tools::{bookir::{CowStr, RichText, RichTextOptions, RichTextParser}, preprocess::{VisitResult, Visitor, preprocess_mdbook_chapter}};

use mdbook_preprocessor::parse_input;
use serde::Deserialize;

struct MacroVisitor<'a> {
    config: &'a MacrosConfig,
    output: &'a str,
    macro_content_cache: HashMap<CowStr<'static>,(&'static str, RichText<'static>)>,
    file: PathBuf,
    root_path: &'a Path,
    implied_files: &'a HashMap<&'static str, &'static Path>,
}

impl<'a> MacroVisitor<'a> {
    fn included(&self, m: &str) -> bool {
        let output = self.config.outputs.get(self.output);
        

        if let Some(output) = output {
            let filter = if let Some(mfilter) = output.by_macro.get(m) {
                mfilter
            } else {
                &output.top_level
            };

            if !filter.test(&self.file) {
                return false;
            }
        }

        let filter = if let Some(mfilter) = self.config.file_spec.by_macro.get(m) {
            mfilter
        } else {
            &self.config.file_spec.top_level
        };

        filter.test(&self.file)
    }
}

impl<'a> Visitor for MacroVisitor<'a> {
    fn visit_node<'b>(&mut self, node: &RichText<'b>) -> VisitResult<RichText<'b>> {
        match node {
            RichText::Paragraph(_) => VisitResult::Interested,
            RichText::BlockQuote(_) => VisitResult::Interested,
            
            RichText::Macro(m) => {
                if !self.included(m) {
                    VisitResult::Update { result: RichText::Comment(m.clone()), recurse: false }
                } else {
                    if let Some((_, rt)) = self.macro_content_cache.get(m) {
                        VisitResult::Update { result: rt.clone(), recurse: true }
                    } else {
                        let mut path = self.root_path.to_path_buf();

                        if let Some(root) = self.config.macro_defs.get(m.as_str()) {
                            path.push(root);
                        } else if let Some(implied) = self.implied_files.get(m.as_str()) {
                            path.push(*implied)
                        } else {
                            eprintln!("Unknown Macro: {m}");
                            return VisitResult::Update { result: RichText::Comment(m.clone()), recurse: false }
                        }

                        let s = match std::fs::read_to_string(&path) {
                            Ok(file) => file,
                            Err(e) => {
                                eprintln!("Expanding Macro {m} raised error (could not read {}): {e}", path.display());
                                return VisitResult::Update { result: RichText::Comment(m.clone()), recurse: false }
                            }
                        };

                        let s = s.leak();

                        let rich = RichTextParser::new(s, self.config.options);

                        let data = rich.collect::<Vec<_>>();

                        let node = RichText::Paragraph(data);

                        self.macro_content_cache.insert(m.clone().into_static(), (s, node.clone()));

                        VisitResult::Update { result: node, recurse: true }
                    }
                }
            },
            _ => mdbook_fiction_tools::preprocess::VisitResult::Ignored,
        }
    }
}



#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct MacrosConfig {
    #[serde(flatten)]
    options: RichTextOptions,
    #[serde(default)]
    macro_defs: HashMap<String, PathBuf>,
    #[serde(flatten)]
    file_spec: FileSpec,
    #[serde(default)]
    outputs: HashMap<String, FileSpec>
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct FileSpec {
    #[serde(flatten)]
    top_level: FileSpecTopLevel,
    #[serde(flatten)]
    by_macro: HashMap<String, FileSpecTopLevel>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct FileSpecTopLevel {
    include_files: Option<HashSet<PathBuf>>,
    exclude_files: HashSet<PathBuf>,
}

impl FileSpecTopLevel {
    pub fn test(&self, v: &Path) -> bool {
        if let Some(include) = &self.include_files {
            if !include.contains(v) {
                return false
            }
        }

        !self.exclude_files.contains(v)
    }
}

const IMPLIED_MACRO_DEFAULT_FILES: &[(&str, &str)] = &[
    ("copyright", "COPYRIGHT-STUD.md")
];

fn main() {

    let mut args = std::env::args();

    let _ = args.next().unwrap();

    for a in args {
        if a == "supports" {
            return;
        }
    }


    let (context, mut input) = parse_input(std::io::stdin()).unwrap();

    let config: MacrosConfig = context.config.get("preprocessor.fiction-macros").unwrap().unwrap_or_default();

    let implied_files = IMPLIED_MACRO_DEFAULT_FILES.iter().copied().map(|(m, p)| (m, Path::new(p))).collect::<HashMap<_, _>>();


    let mut visitor = MacroVisitor { config: &config, output: &context.renderer, macro_content_cache: HashMap::new(), file: PathBuf::new(), root_path: &context.root, implied_files: &implied_files  };

    input.for_each_chapter_mut(|c| {
        if let Some(v) = c.source_path.clone() {
            visitor.file = v;
            preprocess_mdbook_chapter(c, config.options, &mut visitor).unwrap();
        }
    });

    serde_json::to_writer(std::io::stdout(), &input).unwrap();
    
    
}