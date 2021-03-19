use crate::{BoxName, Mp4Box, Mp4BoxError};

use super::{mehd::MovieExtendsHeaderBox, trex::TrackExtendsBox};

use std::io::Write;

pub struct MovieExtendsBox {
    pub mehd: MovieExtendsHeaderBox,
    pub trex: TrackExtendsBox,
}

impl Mp4Box for MovieExtendsBox {
    const NAME: BoxName = *b"mvex";

    fn content_size(&self) -> u64 {
        self.mehd.size() + self.trex.size()
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.mehd.write(writer)?;
        self.trex.write(writer)?;

        Ok(())
    }
}
