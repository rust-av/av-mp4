use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct SyncSampleBox {
    full_box: FullBox,
    pub sync_samples: Vec<u32>,
}

impl SyncSampleBox {
    pub fn new(sync_samples: Vec<u32>) -> Self {
        SyncSampleBox {
            full_box: FullBox::new(*b"stss", 0, 0),
            sync_samples,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read_named(reader, *b"stss")?;

        let count = reader.read_u32::<BigEndian>()?;

        let mut sync_samples = Vec::new();

        for _ in 0..count {
            let sample = reader.read_u32::<BigEndian>()?;

            sync_samples.push(sample);
        }

        Ok(SyncSampleBox {
            full_box,
            sync_samples,
        })
    }

    pub fn write(mut self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u32::<BigEndian>(self.sync_samples.len() as u32)?;

        // convert to BE before writing
        for entry in &mut self.sync_samples {
            *entry = entry.to_be();
        }

        writer.write_all(bytemuck::cast_slice(&self.sync_samples))?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u32>() as u64 + (size_of::<u32>() as u64) * self.sync_samples.len() as u64
    }
}
