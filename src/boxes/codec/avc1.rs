use crate::*;

use super::avcc::AvcConfigurationBox;

use std::io::Write;

pub struct AvcSampleEntryBox {
    pub visual_sample_entry: VisualSampleEntry,
    pub avcc: AvcConfigurationBox,
}

impl AvcSampleEntryBox {
    pub fn new(width: u16, height: u16, avcc: AvcConfigurationBox) -> Self {
        AvcSampleEntryBox {
            visual_sample_entry: VisualSampleEntry::new(*b"avc1", 1, width, height),
            avcc,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.visual_sample_entry.write(writer, self.total_size())?;

        self.avcc.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.visual_sample_entry.size(self.size())
    }

    fn size(&self) -> u64 {
        self.avcc.total_size()
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let visual_sample_entry = VisualSampleEntry::read(buf)?;
        let avcc = AvcConfigurationBox::read(buf)?;

        Ok(AvcSampleEntryBox {
            visual_sample_entry,
            avcc,
        })
    }
}
