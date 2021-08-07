use crate::*;

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
    pub boks: Boks,
    pub media_header: Option<MediaHeader>,
    pub dinf: Option<DataInformationBox>,
    pub stbl: SampleTableBox,
}

impl MediaInformationBox {
    pub fn new(media_header: MediaHeader, dinf: DataInformationBox, stbl: SampleTableBox) -> Self {
        MediaInformationBox {
            boks: Boks::new(*b"minf"),
            media_header: Some(media_header),
            dinf: Some(dinf),
            stbl,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read_named(reader, *b"minf")?;

        let mut stbl = None;

        let iter = BoksIterator::new(reader, boks.remaining_size());
        while let Some((pos, boks)) = iter.next(reader) {
            debug!("{}: {:?}", pos, boks);

            match &boks.name {
                // b"vmhd" => media_header = Some(MediaHeader::Video(VideoMediaHeaderBox::read(reader)?)),
                // b"smhd" => media_header = Some(MediaHeader::Sound(SoundMediaHeaderBox::read(reader)?)),
                // b"dinf" => dinf = Some(DataInformationBox::read(reader)?),
                b"stbl" => stbl = Some(SampleTableBox::read(reader)?),
                _ => {
                    warn!("skipping minf box {:?}", boks);
                    skip(reader, boks.size)?;
                }
            }
        }

        Ok(MediaInformationBox {
            boks,
            media_header: None,
            dinf: None,
            stbl: require_box(stbl, *b"stbl")?,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        match require_box(self.media_header, *b"vmhd")? {
            MediaHeader::Video(vmhd) => vmhd.write(writer)?,
            MediaHeader::Sound(smhd) => smhd.write(writer)?,
        }

        require_box(self.dinf, *b"dinf")?.write(writer)?;
        self.stbl.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        let mut size =
            self.dinf.as_ref().map(|d| d.total_size()).unwrap_or(0) + self.stbl.total_size();

        match &self.media_header {
            Some(MediaHeader::Video(vmhd)) => size += vmhd.total_size(),
            Some(MediaHeader::Sound(smhd)) => size += smhd.total_size(),
            _ => {}
        }

        size
    }
}
