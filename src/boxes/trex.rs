use byteorder::{BigEndian, ByteOrder};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct TrackExtendsBox {
    pub track_id: u32,
    pub default_sample_description_index: u32,
    pub default_sample_duration: u32,
    pub default_sample_size: u32,
    pub default_sample_flags: u32,
}

impl Mp4Box for TrackExtendsBox {
    const NAME: BoxName = *b"trex";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64 + // track_ID
        size_of::<u32>() as u64 + // default_sample_description_index
        size_of::<u32>() as u64 + // default_sample_duration
        size_of::<u32>() as u64 + // default_sample_size
        size_of::<u32>() as u64 // default_sample_flags
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let mut contents = [0u8; 20];

        BigEndian::write_u32(&mut contents[..], self.track_id);
        BigEndian::write_u32(&mut contents[4..], self.default_sample_description_index);
        BigEndian::write_u32(&mut contents[8..], self.default_sample_duration);
        BigEndian::write_u32(&mut contents[12..], self.default_sample_size);
        BigEndian::write_u32(&mut contents[16..], self.default_sample_flags);

        writer.write_all(&contents)?;

        Ok(())
    }
}
