use crate::*;

use super::{mdia::MediaBox, tkhd::TrackHeaderBox};

use std::io::Write;

pub struct TrackBox {
    boks: Boks,
    pub tkhd: TrackHeaderBox,
    pub mdia: MediaBox,
}

impl TrackBox {
    pub fn new(tkhd: TrackHeaderBox, mdia: MediaBox) -> Self {
        TrackBox {
            boks: Boks::new(*b"trak"),
            tkhd,
            mdia,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read_named(reader, *b"trak")?;

        let mut tkhd = None;
        let mut mdia = None;

        let iter = BoksIterator::new(reader, boks.remaining_size());
        while let Some((pos, boks)) = iter.next(reader) {
            debug!("{}: {:?}", pos, boks);

            match &boks.name {
                b"tkhd" => tkhd = Some(TrackHeaderBox::read(reader)?),
                b"mdia" => mdia = Some(MediaBox::read(reader)?),
                _ => {
                    warn!("skipping trak box {:?}", boks);
                    skip(reader, boks.size)?;
                }
            }
        }

        Ok(TrackBox {
            boks,
            tkhd: require_box(tkhd, *b"tkhd")?,
            mdia: require_box(mdia, *b"mdia")?,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        self.tkhd.write(writer)?;
        self.mdia.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        self.tkhd.total_size() + self.mdia.total_size()
    }
}
