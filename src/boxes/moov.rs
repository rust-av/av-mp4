use crate::BoxName;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::{mvex::MovieExtendsBox, mvhd::MovieHeaderBox, trak::TrackBox};

use std::io::Write;

pub struct MovieBox {
    pub mvhd: MovieHeaderBox,
    pub mvex: Option<MovieExtendsBox>,
    pub tracks: Vec<TrackBox>,
}

impl Mp4Box for MovieBox {
    const NAME: BoxName = *b"moov";

    fn content_size(&self) -> u64 {
        let mut size = self.mvhd.size();

        if let Some(mvex) = &self.mvex {
            size += mvex.size();
        }

        for track in &self.tracks {
            size += track.size();
        }

        size
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.mvhd.write(writer)?;

        if let Some(mvex) = self.mvex {
            mvex.write(writer)?;
        }

        for track in self.tracks {
            track.write(writer)?;
        }

        Ok(())
    }
}
