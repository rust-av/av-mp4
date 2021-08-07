use crate::*;

use super::{hdlr::HandlerBox, mdhd::MediaHeaderBox, minf::MediaInformationBox};

use std::io::Write;

pub struct MediaBox {
    pub boks: Boks,
    pub mdhd: MediaHeaderBox,
    pub hdlr: HandlerBox,
    pub minf: MediaInformationBox,
}

impl MediaBox {
    pub fn new(mdhd: MediaHeaderBox, hdlr: HandlerBox, minf: MediaInformationBox) -> Self {
        MediaBox {
            boks: Boks::new(*b"mdia"),
            mdhd,
            hdlr,
            minf,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read_named(reader, *b"mdia")?;

        let mut mdhd = None;
        let mut hdlr = None;
        let mut minf = None;

        let iter = BoksIterator::new(reader, boks.remaining_size());
        while let Some((pos, boks)) = iter.next(reader) {
            debug!("{}: {:?}", pos, boks);

            match &boks.name {
                b"mdhd" => mdhd = Some(MediaHeaderBox::read(reader)?),
                b"hdlr" => hdlr = Some(HandlerBox::read(reader)?),
                b"minf" => minf = Some(MediaInformationBox::read(reader)?),
                _ => {
                    warn!("skipping mdia box {:?}", boks);
                    skip(reader, boks.size)?;
                }
            }
        }

        Ok(MediaBox {
            boks,
            mdhd: require_box(mdhd, *b"mdhd")?,
            hdlr: require_box(hdlr, *b"hdlr")?,
            minf: require_box(minf, *b"minf")?,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        self.mdhd.write(writer)?;
        self.hdlr.write(writer)?;
        self.minf.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        self.mdhd.total_size() + self.hdlr.total_size() + self.minf.total_size()
    }
}
