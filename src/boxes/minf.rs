use crate::BoxName;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::{
    dinf::DataInformationBox, smhd::SoundMediaHeaderBox, stbl::SampleTableBox,
    vmhd::VideoMediaHeaderBox,
};

use std::io::Write;

pub enum MediaHeader {
    Video(VideoMediaHeaderBox),
    Sound(SoundMediaHeaderBox),
}

pub struct MediaInformationBox {
    pub media_header: MediaHeader,
    pub dinf: DataInformationBox,
    pub stbl: SampleTableBox,
}

impl Mp4Box for MediaInformationBox {
    const NAME: BoxName = *b"minf";

    fn content_size(&self) -> u64 {
        let mut size = self.dinf.size() + self.stbl.size();

        match &self.media_header {
            MediaHeader::Video(vmhd) => size += vmhd.size(),
            MediaHeader::Sound(smhd) => size += smhd.size(),
        }

        size
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        match self.media_header {
            MediaHeader::Video(vmhd) => vmhd.write(writer)?,
            MediaHeader::Sound(smhd) => smhd.write(writer)?,
        }

        self.dinf.write(writer)?;
        self.stbl.write(writer)?;

        Ok(())
    }
}
