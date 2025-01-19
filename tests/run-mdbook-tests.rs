#![feature(exit_status_error, os_str_display)]

use std::process::Command;

macro_rules! build_test_fn{
    {$($(#[$meta:meta])* test $($parts:ident)-+;)*} => {
        paste::paste!{
           $(
            $(#[$meta])*
            #[test]
            fn [<run_ $($parts)_* _tests>]() -> std::io::Result<()>{
                const EXE: &str = ::core::env!(::core::concat!("CARGO_BIN_EXE_mdbook",$("-", ::core::stringify!($parts)),+));
                for test in std::fs::read_dir(::core::concat!("tests/mdbook", $("-", ::core::stringify!($parts)),+))?.chain(std::fs::read_dir("tests/common")?)
                {
                    let test = test?;
                    println!(::core::concat!("(" $(,::core::stringify!($parts),)"-"+ "): {}"), test.file_name().display());
                    let path = test.path();
                    Command::new("mdbook")
                        .arg("build")
                        .arg(path)
                        .env(::core::concat!("MDBOOK_output__" $(,::core::stringify!($parts),)"_"+ "__command"), EXE)
                        .status()?
                        .exit_ok()
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                }
                Ok(())
            })*
        }
    }
}
build_test_fn! {
    test bookir;
    #[cfg(feature = "epub")]
    test epub-fancy;
}
