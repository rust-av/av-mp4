use crate::*;

use super::dref::DataReferenceBox;

use std::io::Write;

pub struct DataInformationBox {
    boks: Boks,
    pub dref: DataReferenceBox,
}

impl DataInformationBox {
    pub fn new(dref: DataReferenceBox) -> Self {
        DataInformationBox {
            boks: Boks::new(*b"dinf"),
            dref,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        self.dref.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        self.dref.total_size()
    }
}
