use std::{
    collections::HashMap,
    io::{self, Write},
    rc::Rc,
};

use uuid::Uuid;

use crate::pdf::data::Name;

use super::data::{Dictionary, Object};

pub const HEADER: &str = "%PDF-2.0\n";
pub const UTF8_MARKER: &str = "%🦀\n";
pub const EOF: &str = "%%EOF\n";
pub const XREF_TAB_EOL: &str = "\r\n";
pub const OBJ0_LINE: &str = "0000000000 65535 f";

pub struct PdfWriter<W> {
    curr_offset: u32,
    crossref_table: HashMap<u32, u32>,
    file: W,
    indirect_objects: HashMap<Rc<Object>, u32>,
    next_indirect_index: u32,
    dictionary: Option<Rc<Object>>,
}

impl<W: Write> Write for PdfWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let count = self.file.write(buf)?;
        self.curr_offset = self.curr_offset.checked_add(count as u32).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::StorageFull,
                "A PDF File can contain a maximum of 4GiB from the first character to the last",
            )
        })?;

        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl<W> PdfWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            curr_offset: 0,
            crossref_table: HashMap::new(),
            file: writer,
            indirect_objects: HashMap::new(),
            next_indirect_index: 1,
            dictionary: None,
        }
    }
}

impl<W: io::Write> PdfWriter<W> {
    pub fn write_root_dictionary(&mut self, dict: Rc<Object>) -> io::Result<()> {
        self.write_indirect_object(&dict)?;

        self.dictionary = Some(dict);

        Ok(())
    }
    pub fn write_indirect_object(&mut self, obj: &Rc<Object>) -> io::Result<()> {
        let (index, inline) = if let Some(&idx) = self.indirect_objects.get(obj) {
            (idx, false)
        } else {
            let idx = self.next_indirect_index;
            self.next_indirect_index += 1;
            self.indirect_objects.insert(Rc::clone(obj), idx);

            (idx, true)
        };

        write!(self, "{index} 0 ")?;

        if inline {
            writeln!(self, "object")?;
            obj.write(self)?;
            writeln!(self, "endobject")
        } else {
            write!(self, "R")
        }
    }

    pub fn begin(&mut self) -> io::Result<()> {
        self.write_all(HEADER.as_bytes())?;
        self.write_all(UTF8_MARKER.as_bytes())
    }

    pub fn end(&mut self, id: &Uuid) -> io::Result<()> {
        let beginxref = self.curr_offset;
        writeln!(self, "xref")?;
        writeln!(self, "0 {}", { self.next_indirect_index })?;
        write!(self, "{OBJ0_LINE}{XREF_TAB_EOL}")?;
        let xref = core::mem::take(&mut self.crossref_table);
        for id in 1..self.next_indirect_index {
            let pos = xref[&id];
            write!(self, "{pos:010} 00000 n{XREF_TAB_EOL}")?;
        }
        writeln!(self, "trailer")?;
        let mut trailer_dict = Dictionary::new();

        let root = self.dictionary.take();

        trailer_dict.insert(
            Name::from_str("Size"),
            Object::Integer(self.next_indirect_index),
        );
        let st = id.to_string();
        trailer_dict.insert(
            Name::from_str("ID"),
            Object::Array(vec![Object::String(st.clone()), Object::String(st)]),
        );
        if let Some(root) = root {
            trailer_dict.insert(Name::from_str("Root"), Object::Indirect(root));
        }

        trailer_dict.write(self)?;
        write!(self, "startxref\n{beginxref}\n{EOF}")
    }
}
