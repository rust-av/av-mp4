use av_format::buffer::Buffered;
use av_format::error::Error as AvError;
use av_format::error::Result as AvResult;

use byteorder::{BigEndian, ByteOrder};

use log::*;

use std::fmt;
use std::io::{Error as IoError, Write};

pub mod boxes {
    pub mod vpcc;
    pub mod vpxx;

    pub mod avc1;
    pub mod avcc;

    pub mod dinf;
    pub mod dref;
    pub mod ftyp;
    pub mod hdlr;
    pub mod mdat;
    pub mod mdhd;
    pub mod mdia;
    pub mod mehd;
    pub mod minf;
    pub mod moov;
    pub mod mvex;
    pub mod mvhd;
    pub mod smhd;
    pub mod stbl;
    pub mod tkhd;
    pub mod trak;
    pub mod trex;
    pub mod url;
    pub mod vmhd;

    pub mod co64;
    pub mod stsc;
    pub mod stsd;
    pub mod stss;
    pub mod stsz;
    pub mod stts;
}

pub mod demuxer;
pub mod muxer;

pub struct I16F16(u32);

impl fmt::Debug for I16F16 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let int = self.0 >> 16;
        let frac = self.0 & 0x0000_ffff;

        write!(formatter, "{}.{}", int, frac)
    }
}

impl I16F16 {
    fn raw(&self) -> u32 {
        self.0
    }
}

impl From<u32> for I16F16 {
    fn from(val: u32) -> Self {
        I16F16(val << 16)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Mp4BoxError {
    #[error("I/O error: {0}")]
    Io(#[from] IoError),
}

impl From<Mp4BoxError> for AvError {
    fn from(error: Mp4BoxError) -> AvError {
        match error {
            Mp4BoxError::Io(err) => AvError::Io(err),
        }
    }
}

fn get_total_box_size<B: Mp4Box + ?Sized>(boks: &B) -> u64 {
    let size = boks.content_size();

    size + boks.class().size() + 8
}

fn write_box_header<B: Mp4Box + ?Sized>(header: &mut [u8], size: u64) -> usize {
    if size > u32::MAX as _ {
        BigEndian::write_u32(&mut header[..], 1);
        header[4..8].copy_from_slice(&B::NAME);
        BigEndian::write_u64(&mut header[8..], size);

        16
    } else {
        BigEndian::write_u32(&mut header[..], size as u32);
        header[4..8].copy_from_slice(&B::NAME);

        8
    }
}

fn write_box_class(header: &mut [u8], class: BoxClass) -> usize {
    match class {
        BoxClass::FullBox { version, flags } => {
            header[0] = version;
            BigEndian::write_u24(&mut header[1..], flags);
        }
        BoxClass::SampleEntry {
            data_reference_index,
        } => {
            BigEndian::write_u16(&mut header[6..], data_reference_index);
        }
        BoxClass::VisualSampleEntry {
            data_reference_index,
            width,
            height,
        } => {
            BigEndian::write_u16(&mut header[6..], data_reference_index);
            BigEndian::write_u16(&mut header[24..], width);
            BigEndian::write_u16(&mut header[26..], height);
            BigEndian::write_u32(&mut header[28..], 0x0048_0000);
            BigEndian::write_u32(&mut header[32..], 0x0048_0000);
            BigEndian::write_u16(&mut header[40..], 1);
            BigEndian::write_u16(&mut header[74..], 0x0018);
            BigEndian::write_i16(&mut header[76..], -1);
        }
        _ => {}
    }

    class.size() as usize
}

#[derive(Copy, Clone)]
pub enum BoxClass {
    Box,
    FullBox {
        version: u8,
        flags: u32,
    },
    SampleEntry {
        data_reference_index: u16,
    },
    VisualSampleEntry {
        data_reference_index: u16,
        width: u16,
        height: u16,
    },
}

impl BoxClass {
    const fn max_size() -> usize {
        78
    }

    fn size(&self) -> u64 {
        match self {
            BoxClass::Box => 0,
            BoxClass::FullBox { .. } => 4,
            BoxClass::SampleEntry { .. } => 8,
            BoxClass::VisualSampleEntry { .. } => 78,
        }
    }
}

pub trait Mp4Box {
    const NAME: BoxName;

    fn class(&self) -> BoxClass {
        BoxClass::Box
    }

    fn size(&self) -> u64 {
        get_total_box_size::<Self>(&self)
    }

    fn write_header(size: u64, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let mut header = [0u8; 16];

        if size > u32::MAX as _ {
            BigEndian::write_u32(&mut header[..], 1);
            header[4..8].copy_from_slice(&Self::NAME);
            BigEndian::write_u64(&mut header[8..], size);

            writer.write_all(&header[..])?;
        } else {
            BigEndian::write_u32(&mut header[..], size as u32);
            header[4..8].copy_from_slice(&Self::NAME);

            writer.write_all(&header[..8])?;
        }

        Ok(())
    }

    fn write_class(class: BoxClass, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        let mut cls = [0u8; BoxClass::max_size()];

        let size = write_box_class(&mut cls, class);

        writer.write_all(&cls[..size])?;

        Ok(())
    }

    fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError>
    where
        Self: Sized,
    {
        let mut header = [0u8; 16 + BoxClass::max_size()];

        let mut size = write_box_header::<Self>(&mut header, self.size());
        size += write_box_class(&mut header[size..], self.class()) as usize;
        writer.write_all(&header[..size])?;

        self.write_contents(writer)?;

        Ok(())
    }

    fn content_size(&self) -> u64;
    fn write_contents(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError>;
}

pub type BoxName = [u8; 4];

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct BoxPrint([u8; 4]);

impl fmt::Debug for BoxPrint {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "'{}'", self)
    }
}

impl fmt::Display for BoxPrint {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{}{}{}{}",
            self.0[0] as char, self.0[1] as char, self.0[2] as char, self.0[3] as char,
        )
    }
}

pub fn read_box_header(buf: &mut dyn Buffered) -> AvResult<(BoxName, u64, u64)> {
    let mut size_type = [0u8; 8];
    buf.read_exact(&mut size_type)?;

    let mut size = BigEndian::read_u32(&size_type[..4]) as u64;
    let name = [size_type[4], size_type[5], size_type[6], size_type[7]];

    if size == 1 {
        buf.read_exact(&mut size_type)?;
        size = BigEndian::read_u64(&size_type[..]);

        Ok((name, size, size - 16))
    } else {
        Ok((name, size, size - 8))
    }
}

pub fn read_box_flags(buf: &mut dyn Buffered) -> AvResult<(u8, u32)> {
    let mut val = [0u8; 4];
    buf.read_exact(&mut val)?;

    let version = val[0];
    let flags = BigEndian::read_u24(&val[1..]);

    Ok((version, flags))
}
