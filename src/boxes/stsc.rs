use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

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
    pub entries: Vec<SampleToChunkEntry>,
}

impl Mp4Box for SampleToChunkBox {
    const NAME: BoxName = *b"stsc";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64 + (size_of::<u32>() as u64 * 3) * self.entries.len() as u64
    }

    fn write_contents(mut self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(self.entries.len() as u32)?;

        // convert to BE before writing
        for entry in &mut self.entries {
            *entry = entry.to_be();
        }

        writer.write_all(bytemuck::cast_slice(&self.entries))?;

        Ok(())
    }
}
