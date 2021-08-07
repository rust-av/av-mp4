use byteorder::{BigEndian, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct HandlerBox {
    pub full_box: FullBox,
    pub handler_type: u32,
    pub name: String,
}

impl HandlerBox {
    pub fn new(handler_type: u32, name: String) -> Self {
        HandlerBox {
            full_box: FullBox::new(*b"hdlr", 0, 0),
            handler_type,
            name,
        }
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read(buf)?;

        let mut bytes = [0u8; 20];
        buf.read_exact(&mut bytes)?;

        let handler_type = BigEndian::read_u32(&bytes[4..]);

        let mut name = Vec::new();
        let _ = buf.read_until(b'\0', &mut name)?;

        let name = String::from_utf8(name)?;

        Ok(HandlerBox {
            full_box,
            handler_type,
            name,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u32::<BigEndian>(0)?;
        writer.write_u32::<BigEndian>(self.handler_type)?;
        writer.write_u32::<BigEndian>(0)?;
        writer.write_u32::<BigEndian>(0)?;
        writer.write_u32::<BigEndian>(0)?;
        writer.write_all(self.name.as_bytes())?;
        writer.write_u8(0)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u32>() as u64 + // pre_defined
        size_of::<u32>() as u64 + // handler_type
        size_of::<u32>() as u64 * 3 + // reserved
        self.name.as_bytes().len() as u64 + // name
        1
    }
}
