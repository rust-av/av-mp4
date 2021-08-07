use byteorder::{BigEndian, ByteOrder};

use crate::*;

use std::io::Write;
use std::mem::size_of;

bitflags::bitflags! {
    pub struct TrackHeaderFlags: u32 {
        const ENABLED = 0x000001;
        const IN_MOVIE = 0x000002;
        const IN_PREVIEW = 0x000004;
        const SIZE_IS_ASPECT_RATIO = 0x000008;
    }
}

#[derive(Debug)]
pub struct TrackHeaderBox {
    pub full_box: FullBox,
    pub creation_time: u64,
    pub modification_time: u64,
    pub track_id: u32,
    pub duration: u64,
    pub width: I16F16,
    pub height: I16F16,
}

impl TrackHeaderBox {
    pub fn new(
        flags: TrackHeaderFlags,
        track_id: u32,
        duration: u64,
        width: I16F16,
        height: I16F16,
    ) -> Self {
        TrackHeaderBox {
            full_box: FullBox::new(*b"tkhd", 1, flags.bits()),
            creation_time: 0,
            modification_time: 0,
            track_id,
            duration,
            width,
            height,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        let mut contents = [0u8; 92];

        BigEndian::write_u64(&mut contents[..], self.creation_time);
        BigEndian::write_u64(&mut contents[8..], self.modification_time);
        BigEndian::write_u32(&mut contents[16..], self.track_id);
        BigEndian::write_u64(&mut contents[24..], self.duration);

        BigEndian::write_i32(&mut contents[44..], 0); // volume

        BigEndian::write_i32(&mut contents[48..], 0x00010000);
        BigEndian::write_i32(&mut contents[64..], 0x00010000);
        BigEndian::write_i32(&mut contents[80..], 0x40000000);

        BigEndian::write_u32(&mut contents[84..], self.width.raw()); // width
        BigEndian::write_u32(&mut contents[88..], self.height.raw()); // height

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

    fn read_v0(buf: &mut dyn Buffered, full_box: FullBox) -> Result<Self, Mp4BoxError> {
        let _flags = TrackHeaderFlags::from_bits(full_box.flags).unwrap();

        let mut contents = [0u8; 80];
        buf.read_exact(&mut contents).unwrap();

        let track_id = BigEndian::read_u32(&contents[8..]);
        let duration = BigEndian::read_u32(&contents[16..]) as u64;

        let width = BigEndian::read_u32(&contents[64..]).into();
        let height = BigEndian::read_u32(&contents[68..]).into();

        Ok(TrackHeaderBox {
            full_box,
            creation_time: 0,
            modification_time: 0,
            track_id,
            duration,
            width,
            height,
        })
    }

    fn read_v1(buf: &mut dyn Buffered, full_box: FullBox) -> Result<Self, Mp4BoxError> {
        let _flags = TrackHeaderFlags::from_bits(full_box.flags).unwrap();

        let mut contents = [0u8; 92];
        buf.read_exact(&mut contents).unwrap();

        let track_id = BigEndian::read_u32(&contents[16..]);
        let duration = BigEndian::read_u64(&contents[24..]);

        let width = BigEndian::read_u32(&contents[76..]).into();
        let height = BigEndian::read_u32(&contents[80..]).into();

        Ok(TrackHeaderBox {
            full_box,
            creation_time: 0,
            modification_time: 0,
            track_id,
            duration,
            width,
            height,
        })
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u64>() as u64 + // creation_time
        size_of::<u64>() as u64 + // modification_time
        size_of::<u32>() as u64 + // track_ID
        size_of::<u32>() as u64 + // reserved
        size_of::<u64>() as u64 + // duration
        size_of::<u32>() as u64 * 2 + // reserved
        size_of::<u16>() as u64 + // layer
        size_of::<u16>() as u64 + // alternate_group
        size_of::<u16>() as u64 + // volume
        size_of::<u16>() as u64 + // reserved
        size_of::<i32>() as u64 * 9 + // matrix
        size_of::<u32>() as u64 + // width
        size_of::<u32>() as u64 // height
    }
}
