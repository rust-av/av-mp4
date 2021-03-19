use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct SoundMediaHeaderBox {}

impl Mp4Box for SoundMediaHeaderBox {
    const NAME: BoxName = *b"smhd";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u16>() as u64 + // balance
        size_of::<u16>() as u64 // reserved
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let contents = [0u8; 4];

        writer.write_all(&contents)?;

        Ok(())
    }
}
