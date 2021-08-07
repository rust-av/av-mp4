use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct VideoMediaHeaderBox {
    full_box: FullBox,
}

impl VideoMediaHeaderBox {
    pub fn new() -> Self {
        VideoMediaHeaderBox {
            full_box: FullBox::new(*b"vmhd", 0, 0),
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        debug!("vmhd: {}", self.total_size());
        self.full_box.write(writer, self.total_size())?;

        let contents = [0u8; 8];

        writer.write_all(&contents)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u16>() as u64 + // graphicsmode
        (size_of::<u16>() as u64 * 3) // opcolor
    }
}
