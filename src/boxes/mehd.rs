use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct MovieExtendsHeaderBox {
    pub fragment_duration: u64,
}

impl Mp4Box for MovieExtendsHeaderBox {
    const NAME: BoxName = *b"mehd";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 1,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u64>() as u64 // fragment_duration
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u64::<BigEndian>(self.fragment_duration)?;

        Ok(())
    }
}
