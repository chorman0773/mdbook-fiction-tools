use std::io;

use mdbook::errors::Error as MdError;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor};
use mdbook_fiction_tools::add_copyright::AddCopyrightPreprocessor;

use semver::{Version, VersionReq};

fn main() {
    let mut args = std::env::args();

    args.next();

    let mut supports = if let Some(key) = args.next() {
        if &key == "supports" {
            Some(args.next().expect("I expect an input"))
        } else {
            None
        }
    } else {
        None
    };

    // Users will want to construct their own preprocessor here
    let preprocessor = AddCopyrightPreprocessor {};

    if let Some(sub_args) = supports {
        handle_supports(&preprocessor, &sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), MdError> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let book_version = Version::parse(&ctx.mdbook_version)?;
    let version_req = VersionReq::parse(mdbook::MDBOOK_VERSION)?;

    if !version_req.matches(&book_version) {
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, renderer: &String) -> ! {
    let supported = pre.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}
