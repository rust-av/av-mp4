use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TimeToSampleEntry {
    pub count: u32,
    pub delta: u32,
}

impl TimeToSampleEntry {
    fn to_be(mut self) -> Self {
        self.count = self.count.to_be();
        self.delta = self.delta.to_be();
        self
    }
}

unsafe impl bytemuck::Pod for TimeToSampleEntry {}
unsafe impl bytemuck::Zeroable for TimeToSampleEntry {}

pub struct TimeToSampleBox {
    full_box: FullBox,
    pub entries: Vec<TimeToSampleEntry>,
}

impl TimeToSampleBox {
    pub fn new(entries: Vec<TimeToSampleEntry>) -> Self {
        TimeToSampleBox {
            full_box: FullBox::new(*b"stts", 0, 0),
            entries,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read_named(reader, *b"stts")?;

        let count = reader.read_u32::<BigEndian>()?;

        let mut entries = Vec::new();

        for _ in 0..count {
            let count = reader.read_u32::<BigEndian>()?;
            let delta = reader.read_u32::<BigEndian>()?;

            entries.push(TimeToSampleEntry { count, delta });
        }

        Ok(TimeToSampleBox { full_box, entries })
    }

    pub fn write(mut self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u32::<BigEndian>(self.entries.len() as _)?;

        // convert to BE before writing
        for entry in &mut self.entries {
            *entry = entry.to_be();
        }

        writer.write_all(bytemuck::cast_slice(&self.entries))?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u32>() as u64
            + (size_of::<u32>() as u64 + size_of::<u32>() as u64) * self.entries.len() as u64
    }
}
