use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

pub struct SampleSizeBox {
    pub sample_sizes: Vec<u32>,
}

impl Mp4Box for SampleSizeBox {
    const NAME: BoxName = *b"stsz";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64
            + size_of::<u32>() as u64
            + size_of::<u32>() as u64 * self.sample_sizes.len() as u64
    }

    fn write_contents(mut self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(0)?;
        writer.write_u32::<BigEndian>(self.sample_sizes.len() as u32)?;

        // convert to BE before writing
        for size in &mut self.sample_sizes {
            *size = size.to_be();
        }

        writer.write_all(bytemuck::cast_slice(&self.sample_sizes))?;

        Ok(())
    }
}
