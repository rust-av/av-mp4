use crate::BoxName;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::{hdlr::HandlerBox, mdhd::MediaHeaderBox, minf::MediaInformationBox};

use std::io::Write;

pub struct MediaBox {
    pub mdhd: MediaHeaderBox,
    pub hdlr: HandlerBox,
    pub minf: MediaInformationBox,
}

impl Mp4Box for MediaBox {
    const NAME: BoxName = *b"mdia";

    fn content_size(&self) -> u64 {
        self.mdhd.size() + self.hdlr.size() + self.minf.size()
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.mdhd.write(writer)?;
        self.hdlr.write(writer)?;
        self.minf.write(writer)?;

        Ok(())
    }
}
