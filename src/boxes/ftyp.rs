use byteorder::{BigEndian, WriteBytesExt};

use std::borrow::Cow;
use std::io::Write;
use std::mem::size_of;

use crate::*;

pub struct FileTypeBox<'a> {
    boks: Boks,
    major_brand: BoxName,
    minor_version: u32,
    compatible_brands: Cow<'a, [BoxName]>,
}

impl<'a> FileTypeBox<'a> {
    pub fn new(
        major_brand: BoxName,
        minor_version: u32,
        compatible_brands: Cow<'a, [BoxName]>,
    ) -> Self {
        FileTypeBox {
            boks: Boks::new(*b"ftyp"),
            major_brand,
            minor_version,
            compatible_brands,
        }
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        writer.write_all(&self.major_brand)?;
        writer.write_u32::<BigEndian>(self.minor_version)?;

        for brand in self.compatible_brands.iter() {
            writer.write_all(brand)?;
        }

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        size_of::<u32>() as u64 + // major_brand
        size_of::<u32>() as u64 + // minor_version
        size_of::<u32>() as u64 * self.compatible_brands.len() as u64 // compatible_brands
    }
}
