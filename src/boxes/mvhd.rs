use byteorder::{BigEndian, ByteOrder};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct MovieHeaderBox {
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
}

impl Mp4Box for MovieHeaderBox {
    const NAME: BoxName = *b"mvhd";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 1,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u64>() as u64 + // creation_time
        size_of::<u64>() as u64 + // modification_time
        size_of::<u32>() as u64 + // timescale
        size_of::<u64>() as u64 + // duration
        size_of::<u32>() as u64 + // rate
        size_of::<u16>() as u64 + // volume
        size_of::<u16>() as u64 + // reserved
        size_of::<u32>() as u64 * 2 + // reserved
        size_of::<i32>() as u64 * 9 + // matrix
        size_of::<u32>() as u64 * 6 + // pre_defined
        size_of::<u32>() as u64 // next_track_ID
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let mut contents = [0u8; 108];

        BigEndian::write_u64(&mut contents[..], self.creation_time);
        BigEndian::write_u64(&mut contents[8..], self.modification_time);
        BigEndian::write_u32(&mut contents[16..], self.timescale);
        BigEndian::write_u64(&mut contents[20..], self.duration);

        BigEndian::write_i32(&mut contents[28..], 0x00010000);
        BigEndian::write_i16(&mut contents[32..], 0x0100);

        BigEndian::write_u16(&mut contents[34..], 0);
        BigEndian::write_u64(&mut contents[36..], 0);

        BigEndian::write_i32(&mut contents[44..], 0x00010000);
        BigEndian::write_i32(&mut contents[60..], 0x00010000);
        BigEndian::write_i32(&mut contents[76..], 0x40000000);

        BigEndian::write_u32(&mut contents[104..], 1);

        writer.write_all(&contents)?;

        Ok(())
    }
}
