use byteorder::{BigEndian, ByteOrder};

use crate::BoxName;
use crate::Buffered;
use crate::Mp4Box;
use crate::Mp4BoxError;

use super::avcc::AvcConfigurationBox;

use std::io::Write;
use std::mem::size_of;

pub struct AvcSampleEntryBox {
    pub width: u16,
    pub height: u16,
    pub avcc: AvcConfigurationBox,
}

impl AvcSampleEntryBox {
    pub fn read(buf: &mut dyn Buffered) -> Result<Self, crate::AvError> {
        let mut contents = [0u8; 78];
        buf.read_exact(&mut contents).unwrap();

        let width = BigEndian::read_u16(&contents[24..]);
        let height = BigEndian::read_u16(&contents[26..]);

        let (_name, _total, _remaining) = crate::read_box_header(buf)?;

        let avcc = AvcConfigurationBox::read(buf)?;

        Ok(AvcSampleEntryBox {
            width,
            height,
            avcc,
        })
    }
}

impl Mp4Box for AvcSampleEntryBox {
    const NAME: BoxName = *b"avc1";

    fn content_size(&self) -> u64 {
        size_of::<u8>() as u64 * 6
            + size_of::<u16>() as u64
            + size_of::<u8>() as u64 * 16
            + size_of::<u16>() as u64
            + size_of::<u16>() as u64
            + size_of::<u32>() as u64
            + size_of::<u32>() as u64
            + size_of::<u8>() as u64 * 4
            + size_of::<u16>() as u64
            + size_of::<u8>() as u64 * 32
            + size_of::<u16>() as u64
            + size_of::<i16>() as u64
            + self.avcc.size()
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let mut contents = [0u8; 82];

        BigEndian::write_u16(&mut contents[6..], 1);
        BigEndian::write_u16(&mut contents[24..], self.width);
        BigEndian::write_u16(&mut contents[26..], self.height);
        BigEndian::write_u32(&mut contents[28..], 0x0048_0000);
        BigEndian::write_u32(&mut contents[32..], 0x0048_0000);
        BigEndian::write_u16(&mut contents[36..], 1);
        BigEndian::write_u16(&mut contents[72..], 0x0018);
        BigEndian::write_i16(&mut contents[74..], -1);

        writer.write_all(&contents)?;

        self.avcc.write(writer)?;

        Ok(())
    }
}
