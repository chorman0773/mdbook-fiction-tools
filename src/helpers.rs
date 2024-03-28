use std::{
    borrow::Cow,
    fs::{DirEntry, ReadDir},
    io,
    path::Path,
};

use mdbook::BookItem;

pub fn name_to_id(mut x: &str) -> String {
    if let Some((l, r)) = x.split_once('{') {
        if let Some((_, r)) = r.split_once('#') {
            let (l, _) = r.split_once('}').unwrap();

            let val = l
                .split_once(|c: char| c.is_whitespace())
                .map(|(l, _)| l)
                .unwrap_or(l);

            return val.to_string();
        } else {
            x = l
        }
    }

    let mut s = String::with_capacity(x.len());
    let mut ws_skip = false;

    for c in x.chars() {
        if c.is_alphanumeric() {
            s.extend(c.to_lowercase())
        } else if c.is_whitespace() {
            if !ws_skip {
                s.push('-');
            }
            ws_skip = true;
            continue;
        } else if c == '_' || c == '-' {
            s.push(c)
        }
        ws_skip = false;
    }

    s
}

fn visit_chapters_impl<
    'a,
    S,
    I: Iterator<Item = &'a BookItem>,
    F: FnMut(&BookItem, &mut S) -> io::Result<()>,
    A: FnMut(&mut S),
>(
    iter: &mut I,
    visitor: &mut F,
    after: &mut A,
    state: &mut S,
) -> io::Result<()> {
    for item in iter {
        visitor(item, state)?;
        match item {
            BookItem::Chapter(c) => {
                let mut it = c.sub_items.iter();
                visit_chapters_impl(&mut it, visitor, after, state)?;
                after(state)
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn visit_chapters<
    'a,
    S,
    I: IntoIterator<Item = &'a BookItem>,
    F: FnMut(&BookItem, &mut S) -> io::Result<()>,
    A: FnMut(&mut S),
>(
    it: I,
    mut visitor: F,
    mut after_descent: A,
    mut state: S,
) -> io::Result<S> {
    let mut it = it.into_iter();
    visit_chapters_impl(&mut it, &mut visitor, &mut after_descent, &mut state)?;

    Ok(state)
}

pub struct RecursiveDirectoryIterator {
    stack: Vec<ReadDir>,
}

impl Iterator for RecursiveDirectoryIterator {
    type Item = std::io::Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.stack.last_mut()?;

            if let Some(item) = item.next() {
                let ent = match item {
                    Ok(ent) => ent,
                    Err(e) => return Some(Err(e)),
                };
                let meta = match ent.metadata() {
                    Ok(meta) => meta,
                    Err(e) => return Some(Err(e)),
                };
                let ftype = meta.file_type();
                if ftype.is_dir() {
                    let path = ent.path();
                    let read = match std::fs::read_dir(path) {
                        Ok(read) => read,
                        Err(e) => return Some(Err(e)),
                    };
                    self.stack.push(read);
                } else {
                    return Some(Ok(ent));
                }
            } else {
                self.stack.pop();
            }
        }
    }
}

pub fn read_dir_recursive<P: AsRef<Path>>(path: P) -> std::io::Result<RecursiveDirectoryIterator> {
    std::fs::read_dir(path).map(|it| RecursiveDirectoryIterator { stack: vec![it] })
}

macro_rules! match_media{
    ($expr:expr; $($ext:tt => $media:literal),* $(,)?) => {
        match $expr{
            $(::core::stringify!($ext) => $media,)*
            _ => "application/octet-stream"
        }
    }
}

pub fn media_type_from_file<P: AsRef<Path> + ?Sized>(path: &P) -> Cow<'static, str> {
    let path = path.as_ref();

    match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => {
            with_builtin_macros::with_builtin! {
                let $input = include_from_root!("src/helpers/known-extensions") in {
                    Cow::Borrowed(match_media!{
                        ext; $input
                    })
                }
            }
        }
        None => Cow::Borrowed("application/octet-stream"),
    }
}
