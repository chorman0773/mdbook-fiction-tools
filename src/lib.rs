
#[cfg(feature = "preprocessor")]
pub mod preprocess;

pub mod config;
#[cfg(feature = "epub")]
pub mod epub;
pub mod helpers;

#[cfg(feature = "pdf")]
pub mod pdf;

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

#[cfg(feature = "renderer")]
pub mod renderer;