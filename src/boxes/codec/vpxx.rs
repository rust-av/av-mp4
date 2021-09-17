use crate::*;

use super::vpcc::VpCodecConfigurationBox;

use std::io::Write;

pub struct Vp9SampleEntryBox {
    pub visual_sample_entry: VisualSampleEntry,
    pub vpcc: VpCodecConfigurationBox,
}

impl Vp9SampleEntryBox {
    pub fn new(width: u16, height: u16, vpcc: VpCodecConfigurationBox) -> Self {
        Vp9SampleEntryBox {
            visual_sample_entry: VisualSampleEntry::new(*b"vp09", 1, width, height),
            vpcc,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.visual_sample_entry.write(writer, self.total_size())?;

        self.vpcc.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.visual_sample_entry.size(self.size())
    }

    fn size(&self) -> u64 {
        self.vpcc.total_size()
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let visual_sample_entry = VisualSampleEntry::read(buf)?;
        let vpcc = VpCodecConfigurationBox::read(buf)?;

        Ok(Vp9SampleEntryBox {
            visual_sample_entry,
            vpcc,
        })
    }
}
