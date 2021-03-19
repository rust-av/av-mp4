use byteorder::{BigEndian, ByteOrder};

use crate::{BoxClass, BoxName, Buffered, Mp4Box, Mp4BoxError};

use super::vpcc::VpCodecConfigurationBox;

use log::*;

use std::io::Write;

pub struct Vp9SampleEntryBox {
    pub width: u16,
    pub height: u16,
    pub vpcc: VpCodecConfigurationBox,
}

impl Vp9SampleEntryBox {
    pub fn read(buf: &mut dyn Buffered) -> Result<Self, crate::AvError> {
        let mut contents = [0u8; 78];
        buf.read_exact(&mut contents).unwrap();

        let width = BigEndian::read_u16(&contents[24..]);
        let height = BigEndian::read_u16(&contents[26..]);

        let (name, _total, _remaining) = crate::read_box_header(buf)?;
        info!("{:?}", name);

        let vpcc = VpCodecConfigurationBox::read(buf)?;

        Ok(Vp9SampleEntryBox {
            width,
            height,
            vpcc,
        })
    }
}

impl Mp4Box for Vp9SampleEntryBox {
    const NAME: BoxName = *b"vp09";

    fn class(&self) -> BoxClass {
        BoxClass::VisualSampleEntry {
            data_reference_index: 1,
            width: self.width,
            height: self.height,
        }
    }

    fn content_size(&self) -> u64 {
        self.vpcc.size()
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.vpcc.write(writer)?;

        Ok(())
    }
}
