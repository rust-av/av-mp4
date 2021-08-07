use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct ChunkOffsetBox {
    full_box: FullBox,
    pub chunk_offsets: Vec<u32>,
}

impl ChunkOffsetBox {
    pub fn new(chunk_offsets: Vec<u32>) -> Self {
        ChunkOffsetBox {
            full_box: FullBox::new(*b"stco", 0, 0),
            chunk_offsets,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read_named(reader, *b"stco")?;

        let count = reader.read_u32::<BigEndian>()?;

        let mut chunk_offsets = Vec::new();

        for _ in 0..count {
            let offset = reader.read_u32::<BigEndian>()?;

            chunk_offsets.push(offset);
        }

        Ok(ChunkOffsetBox {
            full_box,
            chunk_offsets,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u32::<BigEndian>(self.chunk_offsets.len() as u32)?;

        for &chunk_offset in &self.chunk_offsets {
            writer.write_u32::<BigEndian>(chunk_offset)?;
        }

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u32>() as u64 + (size_of::<u32>() as u64) * self.chunk_offsets.len() as u64
    }
}
