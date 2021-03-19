use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::{BoxClass, BoxName, Buffered, Mp4Box, Mp4BoxError};

use std::io::Write;
use std::mem::size_of;

pub struct VpCodecConfigurationBox {
    pub config: VpCodecConfigurationRecord,
}

impl VpCodecConfigurationBox {
    pub fn read(buf: &mut dyn Buffered) -> Result<Self, crate::AvError> {
        let (_version, _flags) = crate::read_box_flags(buf).unwrap();

        Ok(VpCodecConfigurationBox {
            config: VpCodecConfigurationRecord::read(buf)?,
        })
    }
}

impl Mp4Box for VpCodecConfigurationBox {
    const NAME: BoxName = *b"vpcC";

    fn class(&self) -> BoxClass {
        BoxClass::FullBox {
            version: 1,
            flags: 0,
        }
    }

    fn content_size(&self) -> u64 {
        self.config.size()
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.config.write(writer)?;

        Ok(())
    }
}

pub struct VpCodecConfigurationRecord {
    pub profile: u8,
    pub level: u8,
    pub bit_depth: u8,
    pub chroma_subsampling: u8,
    pub video_full_range_flags: u8,
    pub colour_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
}

impl VpCodecConfigurationRecord {
    pub fn read(buf: &mut dyn Buffered) -> Result<Self, crate::AvError> {
        let mut header = [0u8; 6];
        buf.read_exact(&mut header)?;

        let profile = header[0];
        let level = header[1];
        let bit_depth = (header[2] & 0b1111_0000) >> 4;
        let chroma_subsampling = (header[2] & 0b0000_1110) >> 1;
        let video_full_range_flags = (header[2] & 0b0000_0001);
        let colour_primaries = header[3];
        let transfer_characteristics = header[4];
        let matrix_coefficients = header[5];

        let initialization_len = buf.read_u16::<BigEndian>()?;
        if initialization_len > 0 {
            panic!("err");
        }

        Ok(VpCodecConfigurationRecord {
            profile,
            level,
            bit_depth,
            chroma_subsampling,
            video_full_range_flags,
            colour_primaries,
            transfer_characteristics,
            matrix_coefficients,
        })
    }

    fn size(&self) -> u64 {
        size_of::<u8>() as u64 * 6 + size_of::<u16>() as u64
    }

    fn write(&self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let header = [
            self.profile,
            self.level,
            (self.bit_depth << 4) | (self.chroma_subsampling << 1) | self.video_full_range_flags,
            self.colour_primaries,
            self.transfer_characteristics,
            self.matrix_coefficients,
        ];

        writer.write_all(&header)?;

        writer.write_u16::<BigEndian>(0)?;

        Ok(())
    }
}
