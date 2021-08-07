use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct SoundMediaHeaderBox {
    full_box: FullBox,
}

impl SoundMediaHeaderBox {
    pub fn new(_timescale: Vec<u32>) -> Self {
        SoundMediaHeaderBox {
            full_box: FullBox::new(*b"smhd", 0, 0),
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        let contents = [0u8; 4];

        writer.write_all(&contents)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u16>() as u64 + // balance
        size_of::<u16>() as u64 // reserved
    }
}
