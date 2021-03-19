use byteorder::{BigEndian, WriteBytesExt};

use crate::Mp4BoxError;
use crate::{BoxClass, BoxName, Mp4Box};

use std::io::Write;
use std::mem::size_of;

use super::url::DataEntryUrlBox;

pub struct DataReferenceBox {
    pub entries: Vec<DataEntryUrlBox>,
}

impl Mp4Box for DataReferenceBox {
    const NAME: BoxName = *b"dref";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 0,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        let mut size = size_of::<u32>() as u64; // entry_count

        for entry in &self.entries {
            size += entry.size();
        }

        size
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_u32::<BigEndian>(self.entries.len() as _)?;

        for entry in self.entries {
            entry.write(writer)?;
        }

        Ok(())
    }
}
