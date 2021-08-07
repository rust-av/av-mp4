use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use super::{avc1, vpxx};

use std::io::Write;
use std::mem::size_of;

pub enum SampleEntry {
    Avc(avc1::AvcSampleEntryBox),
    Vp9(vpxx::Vp9SampleEntryBox),
}

impl SampleEntry {
    fn size(&self) -> u64 {
        match self {
            SampleEntry::Avc(avc1) => avc1.total_size(),
            SampleEntry::Vp9(vp9) => vp9.total_size(),
        }
    }
}

pub struct SampleDescriptionBox {
    full_box: FullBox,
    pub entries: Vec<SampleEntry>,
}

impl SampleDescriptionBox {
    pub fn new(entries: Vec<SampleEntry>) -> Self {
        SampleDescriptionBox {
            full_box: FullBox::new(*b"stsd", 0, 0),
            entries,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read_named(reader, *b"stsd")?;

        let mut count = reader.read_u32::<BigEndian>()?;

        let mut entries = Vec::new();

        let iter = BoksIterator::new(reader, full_box.remaining_size());
        while let Some((pos, boks)) = iter.next(reader) {
            if count == 0 {
                break;
            }

            debug!("{}: {:?}", pos, boks);

            match &boks.name {
                b"avc1" => entries.push(SampleEntry::Avc(avc1::AvcSampleEntryBox::read(reader)?)),
                b"vp09" => entries.push(SampleEntry::Vp9(vpxx::Vp9SampleEntryBox::read(reader)?)),
                _ => {
                    return Err(Mp4BoxError::UnsupportedSampleEntry(BoxPrint(boks.name)));
                }
            }

            count -= 1;
        }

        Ok(SampleDescriptionBox { full_box, entries })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u32::<BigEndian>(self.entries.len() as _)?;

        for entry in self.entries {
            match entry {
                SampleEntry::Avc(avc1) => avc1.write(writer)?,
                SampleEntry::Vp9(vp9) => vp9.write(writer)?,
            }
        }

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        let mut size = size_of::<u32>() as u64;

        for entry in &self.entries {
            size += entry.size();
        }

        size
    }
}
