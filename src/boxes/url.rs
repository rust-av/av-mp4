use byteorder::WriteBytesExt;

use crate::*;

use std::io::Write;

pub struct DataEntryUrlBox {
    full_box: FullBox,
    pub location: String,
}

impl DataEntryUrlBox {
    pub fn new(location: String) -> Self {
        DataEntryUrlBox {
            full_box: FullBox::new(*b"url ", 0, 0x000001),
            location,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_all(&self.location.as_bytes())?;
        writer.write_u8(0)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        self.location.as_bytes().len() as u64 + 1
    }
}
