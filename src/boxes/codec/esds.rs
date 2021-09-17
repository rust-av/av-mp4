use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

const ES_DESCR_TAG: u8 = 0x3;
const DECODER_CONFIG_DESCR_TAG: u8 = 0x4;
const DECODER_SPECIFIC_DESCR_TAG: u8 = 0x5;

#[derive(Debug)]
pub struct DecoderConfigDescriptor {
    pub descriptor: Descriptor,
    pub object_type_indication: u8,

    pub buffer_size_db: u32,
    pub max_bitrate: u32,
    pub avg_bitrate: u32,

    pub decoder_specific: Vec<u8>,
}

impl DecoderConfigDescriptor {
    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let descriptor = Descriptor::read(buf, DECODER_CONFIG_DESCR_TAG)?;

        let object_type_indication = buf.read_u8()?;
        let _ = buf.read_u8()?;
        let buffer_size_db = buf.read_u24::<BigEndian>()?;
        let max_bitrate = buf.read_u32::<BigEndian>()?;
        let avg_bitrate = buf.read_u32::<BigEndian>()?;

        let mut decoder_specific = Vec::new();

        // TODO: maybe parse to supported descriptor directly, instead of storing bytes
        if let Ok(desc) = Descriptor::read(buf, DECODER_SPECIFIC_DESCR_TAG) {
            decoder_specific.resize(desc.remaining_size() as usize, 0);
            buf.read_exact(&mut decoder_specific[..])?;
        }

        Ok(DecoderConfigDescriptor {
            descriptor,
            object_type_indication,
            buffer_size_db,
            max_bitrate,
            avg_bitrate,
            decoder_specific,
        })
    }

    pub fn total_size(&self) -> u64 {
        self.descriptor.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u8> as u64 +
        size_of::<u8> as u64 +
        size_of::<u8> as u64 * 3 +
        size_of::<u32> as u64 +
        size_of::<u32> as u64
    }
}

fn size_of_length(size: u32) -> u32 {
    match size {
        0x0..=0x7F => 1,
        0x80..=0x3FFF => 2,
        0x4000..=0x1FFFFF => 3,
        _ => 4,
    }
}

#[derive(Debug)]
pub struct Descriptor {
    pub tag: u8,
    pub size: u32,
    read_size: u8,
}

impl Descriptor {
    pub fn write(&self, writer: &mut dyn Write, size: u64) -> Result<(), Mp4BoxError> {
        writer.write_u8(self.tag)?;

        todo!();
    }

    pub fn peek(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let bytes = peek(buf, 1)?;
        let tag = bytes[0];

        let mut size = 0u32;
        for i in 0..4 {
            let bytes = peek(buf, 1 + i)?;
            let b = bytes[1 + i];

            size = (size << 7) | (b & 0b0111_1111) as u32;

            if b & 0b1000_0000 == 0 {
                break;
            }
        }

        Ok(Descriptor {
            tag,
            size,
            read_size: 0,
        })
    }

    pub fn read(buf: &mut dyn Buffered, expected: u8) -> Result<Self, Mp4BoxError> {
        let tag = buf.read_u8()?;

        if tag != expected {
            return Err(Mp4BoxError::UnexpectedTag(expected, tag));
        }

        let mut len = 1;
        let mut size = 0u32;
        for _ in 0..4 {
            let b = buf.read_u8()?;
            len += 1;

            size = (size << 7) | (b & 0b0111_1111) as u32;

            if b & 0b1000_0000 == 0 {
                break;
            }
        }

        Ok(Descriptor {
            tag,
            size,
            read_size: len,
        })
    }

    pub fn remaining_size(&self) -> u64 {
        self.size as u64 - self.read_size as u64
    }

    pub fn size(&self, size: u64) -> u64 {
        size_of::<u8> as u64 +
        size_of_length(size as u32) as u64
    }
}

#[derive(Debug)]
pub struct EsDescriptor {
    pub descriptor: Descriptor,
    pub es_id: u16,
    pub decoder_description: DecoderConfigDescriptor,
}

impl EsDescriptor {
    pub fn write(&self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.descriptor.write(writer, self.total_size())?;

        Ok(())
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let descriptor = Descriptor::read(buf, ES_DESCR_TAG)?;
        let es_id = buf.read_u16::<BigEndian>()?;
        let flags = buf.read_u8()?;

        if (flags & 0b0000_0001) != 0 {
            skip(buf, 2)?;
        }

        if (flags & 0b0000_0010) != 0 {
            let len = buf.read_u8()?;
            skip(buf, len as _)?;
        }

        if (flags & 0b0000_0100) != 0 {
            skip(buf, 2)?;
        }

        let decoder_description = DecoderConfigDescriptor::read(buf)?;

        Ok(EsDescriptor {
            descriptor,
            es_id,
            decoder_description,
        })
    }

    pub fn total_size(&self) -> u64 {
        self.descriptor.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u16> as u64 +
        size_of::<u8> as u64 +
        self.decoder_description.total_size()
    }
}

#[derive(Debug)]
pub struct EsdBox {
    full_box: FullBox,
    pub descriptor: EsDescriptor,
}

impl EsdBox {
    pub fn new(descriptor: EsDescriptor) -> Self {
        EsdBox {
            full_box: FullBox::new(*b"esds", 0, 0),
            descriptor,
        }
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let start = pos(buf)?;
        let full_box = FullBox::read_named(buf, *b"esds")?;

        let descriptor = EsDescriptor::read(buf)?;

        goto(buf, start + full_box.boks.size)?;

        Ok(EsdBox {
            full_box,
            descriptor,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        self.descriptor.write(writer)?;

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        self.descriptor.size()
    }
}
