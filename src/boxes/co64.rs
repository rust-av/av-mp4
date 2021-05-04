use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct ChunkLargeOffsetBox {
    pub chunk_offsets: Vec<u64>,
}

impl Mp4Box for ChunkLargeOffsetBox {
    const NAME: BoxName = *b"co64";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64 + (size_of::<u64>() as u64) * self.chunk_offsets.len() as u64
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(self.chunk_offsets.len() as u32)?;

        for &chunk_offset in &self.chunk_offsets {
            writer.write_u64::<BigEndian>(chunk_offset)?;
        }

        Ok(())
    }
}
