use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct VideoMediaHeaderBox {}

impl Mp4Box for VideoMediaHeaderBox {
    const NAME: BoxName = *b"vmhd";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 1,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u16>() as u64 + // graphicsmode
        (size_of::<u16>() as u64 * 3) // opcolor
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let contents = [0u8; 8];

        writer.write_all(&contents)?;

        Ok(())
    }
}
