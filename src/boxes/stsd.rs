use byteorder::{BigEndian, WriteBytesExt};

use crate::BoxClass;
use crate::BoxName;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::{avc1::AvcSampleEntryBox, vpxx::Vp9SampleEntryBox};

use std::io::Write;
use std::mem::size_of;

pub enum SampleEntry {
    Avc(AvcSampleEntryBox),
    Vp9(Vp9SampleEntryBox),
}

impl SampleEntry {
    fn size(&self) -> u64 {
        match self {
            SampleEntry::Avc(avc1) => avc1.size(),
            SampleEntry::Vp9(vp9) => vp9.size(),
        }
    }
}

pub struct SampleDescriptionBox {
    pub entries: Vec<SampleEntry>,
}

impl Mp4Box for SampleDescriptionBox {
    const NAME: BoxName = *b"stsd";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        let mut size = size_of::<u32>() as u64;

        for entry in &self.entries {
            size += entry.size();
        }

        size
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(self.entries.len() as _)?;

        for entry in self.entries {
            match entry {
                SampleEntry::Avc(avc1) => avc1.write(writer)?,
                SampleEntry::Vp9(vp9) => vp9.write(writer)?,
            }
        }

        Ok(())
    }
}
