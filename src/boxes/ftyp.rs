use byteorder::{BigEndian, WriteBytesExt};

use std::borrow::Cow;
use std::io::Write;
use std::mem::size_of;

use crate::{BoxName, Mp4Box, Mp4BoxError};

pub struct FileTypeBox<'a> {
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
            major_brand,
            minor_version,
            compatible_brands,
        }
    }
}

impl<'a> Mp4Box for FileTypeBox<'a> {
    const NAME: BoxName = *b"ftyp";

    fn content_size(&self) -> u64 {
        size_of::<u32>() as u64 + // major_brand
        size_of::<u32>() as u64 + // minor_version
        size_of::<u32>() as u64 * self.compatible_brands.len() as u64 // compatible_brands
    }

    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        writer.write_all(&self.major_brand)?;
        writer.write_u32::<BigEndian>(self.minor_version)?;

        for brand in self.compatible_brands.iter() {
            writer.write_all(brand)?;
        }

        Ok(())
    }
}
