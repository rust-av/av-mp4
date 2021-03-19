use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct HandlerBox {
    pub handler_type: u32,
    pub name: String,
}

impl Mp4Box for HandlerBox {
    const NAME: BoxName = *b"hdlr";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64 + // pre_defined
        size_of::<u32>() as u64 + // handler_type
        size_of::<u32>() as u64 * 3 + // reserved
        self.name.as_bytes().len() as u64 + // name
        1
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(0)?;
        writer.write_u32::<BigEndian>(self.handler_type)?;
        writer.write_u32::<BigEndian>(0)?;
        writer.write_u32::<BigEndian>(0)?;
        writer.write_u32::<BigEndian>(0)?;
        writer.write_all(self.name.as_bytes())?;
        writer.write_u8(0)?;

        Ok(())
    }
}
