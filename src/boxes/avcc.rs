use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub struct AvcConfigurationBox {
    boks: Boks,
    pub config: AvcDecoderConfigurationRecord,
}

impl AvcConfigurationBox {
    pub fn new(config: AvcDecoderConfigurationRecord) -> Self {
        AvcConfigurationBox {
            boks: Boks::new(*b"avcC"),
            config,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        self.config.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        self.config.size()
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read_named(buf, *b"avcC")?;

        Ok(AvcConfigurationBox {
            boks,
            config: AvcDecoderConfigurationRecord::read(buf)?,
        })
    }
}

pub struct SequenceParameterSet(pub Vec<u8>);
pub struct PictureParameterSet(pub Vec<u8>);

pub struct AvcDecoderConfigurationRecord {
    pub profile_indication: u8,
    pub profile_compatibility: u8,
    pub level_indication: u8,
    pub sequence_parameter_sets: Vec<SequenceParameterSet>,
    pub picture_parameter_sets: Vec<PictureParameterSet>,
}

impl AvcDecoderConfigurationRecord {
    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let mut header = [0u8; 6];
        buf.read_exact(&mut header)?;

        let _length_minus_one = header[4];
        let sps_count = header[5] & 0b0001_1111;
        debug!("sps_count: {:08b}", sps_count);

        let mut sequence_parameter_sets = Vec::new();
        let mut picture_parameter_sets = Vec::new();

        for _i in 0..sps_count {
            let sps_len = buf.read_u16::<BigEndian>()?;
            debug!("sps_len: {}", sps_len);
            let mut sps = vec![0u8; sps_len as usize];

            buf.read_exact(&mut sps)?;

            sequence_parameter_sets.push(SequenceParameterSet(sps));
        }

        let pps_count = buf.read_u8()?;
        debug!("pps_count: {}", pps_count);

        for _i in 0..pps_count {
            let pps_len = buf.read_u16::<BigEndian>()?;
            debug!("pps_len: {}", pps_len);
            let mut pps = vec![0u8; pps_len as usize];

            buf.read_exact(&mut pps)?;

            picture_parameter_sets.push(PictureParameterSet(pps));
        }

        Ok(AvcDecoderConfigurationRecord {
            profile_indication: header[1],
            profile_compatibility: header[2],
            level_indication: header[3],
            sequence_parameter_sets,
            picture_parameter_sets,
        })
    }

    fn size(&self) -> u64 {
        size_of::<u8>() as u64 // configurationVersion
            + size_of::<u8>() as u64 // AVCProfileIndication
            + size_of::<u8>() as u64 // profile_compatibility
            + size_of::<u8>() as u64 // AVCLevelIndication
            + size_of::<u8>() as u64 // lengthSizeMinusOne
            + size_of::<u8>() as u64 // numOfSequenceParameterSets
            + size_of::<u16>() as u64 // sequenceParameterSetLength
            + self.sequence_parameter_sets.iter().map(|sps| sps.0.len() as u64).sum::<u64>()
            + size_of::<u8>() as u64 // numOfPictureParameterSets
            + size_of::<u16>() as u64 // pictureParameterSetLength
            + self.picture_parameter_sets.iter().map(|pps| pps.0.len() as u64).sum::<u64>()
    }

    fn write(&self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let header = [
            1,
            self.profile_indication,
            self.profile_compatibility,
            self.level_indication,
            0b1111_1100 | 3,
            0b1110_0000 | 1,
        ];

        writer.write_all(&header)?;
        writer.write_u16::<BigEndian>(self.sequence_parameter_sets.len() as u16)?;
        for sps in &self.sequence_parameter_sets {
            writer.write_all(&sps.0)?;
        }

        writer.write_u8(1)?;
        writer.write_u16::<BigEndian>(self.picture_parameter_sets.len() as u16)?;
        for pps in &self.picture_parameter_sets {
            writer.write_all(&pps.0)?;
        }

        Ok(())
    }
}
