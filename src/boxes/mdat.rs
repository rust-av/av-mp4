use std::borrow::Cow;
use std::io::Write;

use crate::{BoxName, Mp4Box, Mp4BoxError};

pub struct MediaDataBox<'a> {
    data: Cow<'a, [u8]>,
}

impl<'a> MediaDataBox<'a> {
    pub fn new(data: Cow<'a, [u8]>) -> Self {
        MediaDataBox { data }
    }
}

impl<'a> Mp4Box for MediaDataBox<'a> {
    const NAME: BoxName = *b"mdat";

    fn content_size(&self) -> u64 {
        self.data.len() as u64 // data
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_all(&self.data)?;

        Ok(())
    }
}
