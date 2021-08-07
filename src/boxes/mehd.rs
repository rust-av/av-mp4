use byteorder::{BigEndian, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct MovieExtendsHeaderBox {
    full_box: FullBox,
    fragment_duration: u64,
}

impl MovieExtendsHeaderBox {
    pub fn new(fragment_duration: u64) -> Self {
        MovieExtendsHeaderBox {
            full_box: FullBox::new(*b"mehd", 1, 0),
            fragment_duration,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u64::<BigEndian>(self.fragment_duration)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u64>() as u64 // fragment_duration
    }
}
