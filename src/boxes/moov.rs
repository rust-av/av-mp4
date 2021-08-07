use crate::*;

use super::{mvex::MovieExtendsBox, mvhd::MovieHeaderBox, trak::TrackBox};

use std::io::Write;

pub struct MovieBox {
    boks: Boks,
    pub mvhd: MovieHeaderBox,
    pub mvex: Option<MovieExtendsBox>,
    pub tracks: Vec<TrackBox>,
}

impl MovieBox {
    pub fn new(mvhd: MovieHeaderBox, mvex: Option<MovieExtendsBox>, tracks: Vec<TrackBox>) -> Self {
        MovieBox {
            boks: Boks::new(*b"moov"),
            mvhd,
            mvex,
            tracks,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;
        self.mvhd.write(writer)?;

        if let Some(mvex) = self.mvex {
            mvex.write(writer)?;
        }

        for track in self.tracks {
            track.write(writer)?;
        }

        Ok(())
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read_named(reader, *b"moov")?;

        let mut mvhd = None;
        let mut mvex = None;
        let mut tracks = Vec::new();

        let iter = BoksIterator::new(reader, boks.remaining_size());
        while let Some((pos, boks)) = iter.next(reader) {
            debug!("{}: {:?}", pos, boks);

            match &boks.name {
                b"mvhd" => mvhd = Some(MovieHeaderBox::read(reader)?),
                b"mvex" => mvex = Some(MovieExtendsBox::read(reader)?),
                b"trak" => match TrackBox::read(reader) {
                    Ok(track) => tracks.push(track),
                    Err(e) => {
                        warn!("Failed to parse trak: {}", e);
                        goto(reader, pos + boks.size)?;
                    }
                },
                _ => {
                    warn!("skipping moov box {:?}", boks);
                    skip(reader, boks.size)?;
                }
            }

            debug!("{}", pos);
        }

        Ok(MovieBox {
            boks,
            mvhd: require_box(mvhd, *b"mvhd")?,
            mvex,
            tracks: non_empty(tracks, *b"trak")?,
        })
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        let mut size = self.mvhd.total_size();

        if let Some(mvex) = &self.mvex {
            size += mvex.total_size();
        }

        for track in &self.tracks {
            size += track.total_size();
        }

        size
    }
}
