use std::{
    collections::HashMap,
    io::{self, Write},
    rc::Rc,
};

use super::data::Object;

pub const HEADER: &str = "%PDF-2.0\n";
pub const UTF8_MARKER: &str = "%🦀\n";
pub const EOF: &str = "%%EOF\n";

pub struct PdfWriter<W> {
    curr_offset: u32,
    crossref_table: HashMap<u32, u32>,
    file: W,
    indirect_objects: HashMap<Rc<Object>, u32>,
    next_indirect_index: u32,
}

impl<W: Write> Write for PdfWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let count = self.file.write(buf)?;
        self.curr_offset += count as u32;

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
            next_indirect_index: 0,
        }
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

        Ok(())
    }
}
