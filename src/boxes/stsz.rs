use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::*;

use std::io::Write;
use std::mem::size_of;

pub enum SampleSizes {
    Constant(u32),
    Variable(Vec<u32>),
}

impl SampleSizes {
    fn size(&self) -> u64 {
        match self {
            SampleSizes::Constant(_) => size_of::<u32>() as u64 * 2,
            SampleSizes::Variable(sizes) => size_of::<u32>() as u64 * (2 + sizes.len() as u64),
        }
    }
}

pub struct SampleSizeBox {
    full_box: FullBox,
    pub sample_sizes: SampleSizes,
}

impl SampleSizeBox {
    pub fn new(sample_sizes: SampleSizes) -> Self {
        SampleSizeBox {
            full_box: FullBox::new(*b"stsz", 0, 0),
            sample_sizes,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let full_box = FullBox::read_named(reader, *b"stsz")?;

        let constant_size = reader.read_u32::<BigEndian>()?;
        let count = reader.read_u32::<BigEndian>()?;

        let sample_sizes = if count == 0 {
            SampleSizes::Constant(constant_size)
        } else {
            let mut sample_sizes = Vec::new();

            for _ in 0..count {
                let size = reader.read_u32::<BigEndian>()?;

                sample_sizes.push(size);
            }

            SampleSizes::Variable(sample_sizes)
        };

        Ok(SampleSizeBox {
            full_box,
            sample_sizes,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.full_box.write(writer, self.total_size())?;

        match self.sample_sizes {
            SampleSizes::Constant(constant) => {
                writer.write_u32::<BigEndian>(constant)?;
                writer.write_u32::<BigEndian>(0)?;
            }
            SampleSizes::Variable(mut sizes) => {
                writer.write_u32::<BigEndian>(0)?;
                writer.write_u32::<BigEndian>(sizes.len() as u32)?;

                // convert to BE before writing
                for size in &mut sizes {
                    *size = size.to_be();
                }

                writer.write_all(bytemuck::cast_slice(&sizes))?;
            }
        }

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.full_box.size(self.size())
    }

    fn size(&self) -> u64 {
        self.sample_sizes.size()
    }
}
