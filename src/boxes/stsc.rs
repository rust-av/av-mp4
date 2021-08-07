use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SampleToChunkEntry {
    pub first_chunk: u32,
    pub samples_per_chunk: u32,
    pub sample_description_index: u32,
}

unsafe impl bytemuck::Pod for SampleToChunkEntry {}
unsafe impl bytemuck::Zeroable for SampleToChunkEntry {}

impl SampleToChunkEntry {
    fn to_be(mut self) -> Self {
        self.first_chunk = self.first_chunk.to_be();
        self.samples_per_chunk = self.samples_per_chunk.to_be();
        self.sample_description_index = self.sample_description_index.to_be();
        self
    }
}

pub struct SampleToChunkBox {
    full_box: FullBox,
    pub entries: Vec<SampleToChunkEntry>,
}

impl SampleToChunkBox {
    pub fn new(entries: Vec<SampleToChunkEntry>) -> Self {
        SampleToChunkBox {
            full_box: FullBox::new(*b"stsc", 0, 0),
            entries,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read_named(reader, *b"stsc")?;

        let count = reader.read_u32::<BigEndian>()?;

        let mut entries = Vec::new();

        for _ in 0..count {
            let first_chunk = reader.read_u32::<BigEndian>()?;
            let samples_per_chunk = reader.read_u32::<BigEndian>()?;
            let sample_description_index = reader.read_u32::<BigEndian>()?;

            entries.push(SampleToChunkEntry {
                first_chunk,
                samples_per_chunk,
                sample_description_index,
            });
        }

        Ok(SampleToChunkBox { full_box, entries })
    }

    pub fn write(mut self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;

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
        size_of::<u32>() as u64 + (size_of::<u32>() as u64 * 3) * self.entries.len() as u64
    }
}
