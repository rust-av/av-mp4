use std::borrow::Cow;
use std::io::Write;

use crate::*;

pub struct MediaDataBox<'a> {
    boks: Boks,
    data: Cow<'a, [u8]>,
}

impl<'a> MediaDataBox<'a> {
    pub fn new(data: Cow<'a, [u8]>) -> Self {
        MediaDataBox {
            boks: Boks::new(*b"mdat"),
            data,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        writer.write_all(&self.data)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        self.data.len() as u64 // data
    }
}
