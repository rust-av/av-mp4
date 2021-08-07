use byteorder::{BigEndian, ByteOrder};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct MovieHeaderBox {
    full_box: FullBox,
    creation_time: u64,
    modification_time: u64,
    timescale: u32,
    duration: u64,
}

impl MovieHeaderBox {
    pub fn new(timescale: u32, duration: u64) -> Self {
        MovieHeaderBox {
            full_box: FullBox::new(*b"mvhd", 1, 0),
            creation_time: 0,
            modification_time: 0,
            timescale,
            duration,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read_named(reader, *b"mvhd")?;

        match full_box.version {
            0 => Self::read_v0(reader, full_box),
            1 => Self::read_v1(reader, full_box),
            _ => todo!(),
        }
    }

    pub fn read_v0(reader: &mut dyn Buffered, full_box: FullBox) -> Result<Self, Mp4BoxError> {
        let mut contents = [0u8; 96];
        reader.read_exact(&mut contents)?;

        let timescale = BigEndian::read_u32(&contents[8..]);
        let duration = BigEndian::read_u32(&contents[12..]) as u64;

        Ok(MovieHeaderBox {
            full_box,
            creation_time: 0,
            modification_time: 0,
            timescale,
            duration,
        })
    }

    pub fn read_v1(reader: &mut dyn Buffered, full_box: FullBox) -> Result<Self, Mp4BoxError> {
        let mut contents = [0u8; 108];
        reader.read_exact(&mut contents)?;

        let timescale = BigEndian::read_u32(&contents[16..]);
        let duration = BigEndian::read_u64(&contents[20..]);

        Ok(MovieHeaderBox {
            full_box,
            creation_time: 0,
            modification_time: 0,
            timescale,
            duration,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

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

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
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
}
