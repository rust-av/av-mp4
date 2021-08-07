use byteorder::{BigEndian, ByteOrder};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct TrackExtendsBox {
    full_box: FullBox,
    track_id: u32,
    default_sample_description_index: u32,
    default_sample_duration: u32,
    default_sample_size: u32,
    default_sample_flags: u32,
}

impl TrackExtendsBox {
    pub fn new(
        track_id: u32,
        default_sample_description_index: u32,
        default_sample_duration: u32,
        default_sample_size: u32,
        default_sample_flags: u32,
    ) -> Self {
        TrackExtendsBox {
            full_box: FullBox::new(*b"trex", 0, 0),
            track_id,
            default_sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        let mut contents = [0u8; 20];

        BigEndian::write_u32(&mut contents[..], self.track_id);
        BigEndian::write_u32(&mut contents[4..], self.default_sample_description_index);
        BigEndian::write_u32(&mut contents[8..], self.default_sample_duration);
        BigEndian::write_u32(&mut contents[12..], self.default_sample_size);
        BigEndian::write_u32(&mut contents[16..], self.default_sample_flags);

        writer.write_all(&contents)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u32>() as u64 + // track_ID
        size_of::<u32>() as u64 + // default_sample_description_index
        size_of::<u32>() as u64 + // default_sample_duration
        size_of::<u32>() as u64 + // default_sample_size
        size_of::<u32>() as u64 // default_sample_flags
    }
}
