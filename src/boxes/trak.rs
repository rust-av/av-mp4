use crate::BoxName;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::{mdia::MediaBox, tkhd::TrackHeaderBox};

use std::io::Write;

pub struct TrackBox {
    pub tkhd: TrackHeaderBox,
    pub mdia: MediaBox,
}

impl Mp4Box for TrackBox {
    const NAME: BoxName = *b"trak";

    fn content_size(&self) -> u64 {
        self.tkhd.size() + self.mdia.size()
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.tkhd.write(writer)?;
        self.mdia.write(writer)?;

        Ok(())
    }
}
