use crate::*;

use super::esds::EsdBox;

use std::io::Write;

#[derive(Debug)]
pub struct Mpeg4VideoSampleEntryBox {
    pub visual_sample_entry: VisualSampleEntry,
    pub esds: EsdBox,
}

impl Mpeg4VideoSampleEntryBox {
    pub fn new(width: u16, height: u16, esds: EsdBox) -> Self {
        Mpeg4VideoSampleEntryBox {
            visual_sample_entry: VisualSampleEntry::new(*b"mp4v", 1, width, height),
            esds,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.visual_sample_entry.write(writer, self.total_size())?;

        self.esds.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.visual_sample_entry.size(self.size())
    }

    fn size(&self) -> u64 {
        self.esds.total_size()
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let visual_sample_entry = VisualSampleEntry::read(buf)?;
        let esds = EsdBox::read(buf)?;

        Ok(Mpeg4VideoSampleEntryBox {
            visual_sample_entry,
            esds,
        })
    }
}
