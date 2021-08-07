use byteorder::{BigEndian, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

use super::url::DataEntryUrlBox;

pub struct DataReferenceBox {
    full_box: FullBox,
    pub entries: Vec<DataEntryUrlBox>,
}

impl DataReferenceBox {
    pub fn new(entries: Vec<DataEntryUrlBox>) -> Self {
        DataReferenceBox {
            full_box: FullBox::new(*b"dref", 0, 0),
            entries,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        writer.write_u32::<BigEndian>(self.entries.len() as _)?;

        for entry in self.entries {
            entry.write(writer)?;
        }

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        let mut size = size_of::<u32>() as u64; // entry_count

        for entry in &self.entries {
            size += entry.total_size();
        }

        size
    }
}
