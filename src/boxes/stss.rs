use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct SyncSampleBox {
    pub sync_samples: Vec<u32>,
}

impl Mp4Box for SyncSampleBox {
    const NAME: BoxName = *b"stss";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64 + (size_of::<u32>() as u64) * self.sync_samples.len() as u64
    }

    fn write_contents(mut self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(self.sync_samples.len() as u32)?;

        // convert to BE before writing
        for entry in &mut self.sync_samples {
            *entry = entry.to_be();
        }

        writer.write_all(bytemuck::cast_slice(&self.sync_samples))?;

        Ok(())
    }
}
