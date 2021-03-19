use crate::BoxName;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::{
    co64::ChunkLargeOffsetBox, stsc::SampleToChunkBox, stsd::SampleDescriptionBox,
    stss::SyncSampleBox, stsz::SampleSizeBox, stts::TimeToSampleBox,
};

use std::io::Write;

pub struct SampleTableBox {
    pub stsd: SampleDescriptionBox,
    pub stts: TimeToSampleBox,
    pub stsc: SampleToChunkBox,
    pub stsz: SampleSizeBox,
    pub co64: ChunkLargeOffsetBox,
    pub stss: Option<SyncSampleBox>,
}

impl Mp4Box for SampleTableBox {
    const NAME: BoxName = *b"stbl";

    fn content_size(&self) -> u64 {
        self.stsd.size()
            + self.stts.size()
            + self.stsc.size()
            + self.stsz.size()
            + self.co64.size()
            + self.stss.as_ref().map(|b| b.size()).unwrap_or(0)
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.stsd.write(writer)?;
        self.stts.write(writer)?;
        self.stsc.write(writer)?;
        self.stsz.write(writer)?;
        self.co64.write(writer)?;
        if let Some(stss) = self.stss {
            stss.write(writer)?;
        }

        Ok(())
    }
}
