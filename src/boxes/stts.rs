use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

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
    pub entries: Vec<TimeToSampleEntry>,
}

impl Mp4Box for TimeToSampleBox {
    const NAME: BoxName = *b"stts";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64
            + (size_of::<u32>() as u64 + size_of::<u32>() as u64) * self.entries.len() as u64
    }

    fn write_contents(mut self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(self.entries.len() as _)?;

        // convert to BE before writing
        for entry in &mut self.entries {
            *entry = entry.to_be();
        }

        writer.write_all(bytemuck::cast_slice(&self.entries))?;

        Ok(())
    }
}
