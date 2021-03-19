use crate::BoxName;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::dref::DataReferenceBox;

use std::io::Write;

pub struct DataInformationBox {
    pub dref: DataReferenceBox,
}

impl Mp4Box for DataInformationBox {
    const NAME: BoxName = *b"dinf";

    fn content_size(&self) -> u64 {
        self.dref.size()
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.dref.write(writer)?;

        Ok(())
    }
}
