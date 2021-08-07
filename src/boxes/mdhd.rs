use byteorder::{BigEndian, ByteOrder};

use crate::*;

use std::io::Write;
use std::mem::size_of;

#[derive(Debug)]
pub struct MediaHeaderBox {
    pub full_box: FullBox,
    pub creation_time: u64,
    pub modification_time: u64,
    pub timescale: u32,
    pub duration: u64,
}

impl MediaHeaderBox {
    pub fn new(timescale: u32, duration: u64) -> Self {
        MediaHeaderBox {
            full_box: FullBox::new(*b"mdhd", 1, 0),
            creation_time: 0,
            modification_time: 0,
            timescale,
            duration,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        let mut contents = [0u8; 32];

        BigEndian::write_u64(&mut contents[..], self.creation_time);
        BigEndian::write_u64(&mut contents[8..], self.modification_time);
        BigEndian::write_u32(&mut contents[16..], self.timescale);
        BigEndian::write_u64(&mut contents[20..], self.duration);

        BigEndian::write_u16(&mut contents[28..], 0); // language

        writer.write_all(&contents)?;

        Ok(())
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read(buf)?;

        match full_box.version {
            0 => Self::read_v0(buf, full_box),
            1 => Self::read_v1(buf, full_box),
            _ => todo!(),
        }
    }

    pub fn read_v0(buf: &mut dyn Buffered, full_box: FullBox) -> Result<Self, Mp4BoxError> {
        let mut contents = [0u8; 20];
        buf.read_exact(&mut contents).unwrap();

        let creation_time = BigEndian::read_u32(&contents[0..]) as u64;
        let modification_time = BigEndian::read_u32(&contents[4..]) as u64;
        let timescale = BigEndian::read_u32(&contents[8..]);
        let duration = BigEndian::read_u32(&contents[12..]) as u64;

        Ok(MediaHeaderBox {
            full_box,
            creation_time,
            modification_time,
            timescale,
            duration,
        })
    }

    pub fn read_v1(buf: &mut dyn Buffered, full_box: FullBox) -> Result<Self, Mp4BoxError> {
        let mut contents = [0u8; 32];
        buf.read_exact(&mut contents).unwrap();

        let creation_time = BigEndian::read_u64(&contents[0..]);
        let modification_time = BigEndian::read_u64(&contents[8..]);
        let timescale = BigEndian::read_u32(&contents[16..]);
        let duration = BigEndian::read_u64(&contents[20..]);

        Ok(MediaHeaderBox {
            full_box,
            creation_time,
            modification_time,
            timescale,
            duration,
        })
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u64>() as u64 + // creation_time
        size_of::<u64>() as u64 + // modification_time
        size_of::<u32>() as u64 + // timescale
        size_of::<u64>() as u64 + // duration
        size_of::<u16>() as u64 + // language
        size_of::<u16>() as u64 // pre_defined
    }
}
