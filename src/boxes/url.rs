use byteorder::WriteBytesExt;

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;

pub struct DataEntryUrlBox {
    pub location: String,
}

impl Mp4Box for DataEntryUrlBox {
    const NAME: BoxName = *b"url ";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0x000001,
        }
    }

    fn content_size(&self) -> u64 {
        self.location.as_bytes().len() as u64 + 1
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_all(&self.location.as_bytes())?;
        writer.write_u8(0)?;

        Ok(())
    }
}
