#![feature(exit_status_error, os_str_display)]

use std::process::Command;

const MDBOOK_EPUB_FANCY: &str = std::env!("CARGO_BIN_EXE_mdbook-epub-fancy");

#[test]
fn run_epub_fancy_tests() -> std::io::Result<()> {
    for test in std::fs::read_dir("tests/mdbook-epub-fancy")? {
        let test = test?;
        println!("(epub-fancy): {}", test.file_name().display());
        let path = test.path();
        Command::new("mdbook")
            .arg("build")
            .arg(path)
            .env("MDBOOK_output__epub_fancy__command", MDBOOK_EPUB_FANCY)
            .status()?
            .exit_ok()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }
    Ok(())
}
