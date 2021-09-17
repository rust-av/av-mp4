use av_format::buffer::Buffered;
use av_format::error::Error as AvError;
use av_format::error::Result as AvResult;

use byteorder::{BigEndian, ByteOrder};

use log::*;

use std::fmt;
use std::io::{Error as IoError, SeekFrom, Write};
use std::string::FromUtf8Error;

pub mod boxes {
    pub mod codec {
        pub mod stsd;
        pub mod esds;

        pub mod vpcc;
        pub mod vpxx;

        pub mod avc1;
        pub mod avcc;

        pub mod mp4v;
    }

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
    pub mod stco;
    pub mod stsc;
    pub mod stss;
    pub mod stsz;
    pub mod stts;
}

pub mod demuxer;
pub mod muxer;

pub struct BoksIterator {
    size: u64,
    start: u64,
}

impl BoksIterator {
    pub fn new(reader: &mut dyn Buffered, size: u64) -> Self {
        let start = reader.seek(SeekFrom::Current(0)).unwrap();

        BoksIterator { size, start }
    }

    fn next(&self, reader: &mut dyn Buffered) -> Option<(u64, Boks)> {
        let pos = reader.seek(SeekFrom::Current(0)).unwrap();

        if pos - self.start >= self.size {
            return None;
        }

        let boks = Boks::peek(reader).ok()?;

        Some((pos, boks))
    }
}

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

    #[error("Invalid UTF-8: {0}")]
    InvalidUtf8(#[from] FromUtf8Error),

    #[error("Failed parsing MPEG bitstream: {0}")]
    MpegError(#[from] mpeg1::MpegError),

    #[error("Unexpected end of stream")]
    UnexpectedEos,

    #[error("Unsupported sample entry {0:?}")]
    UnsupportedSampleEntry(BoxPrint),

    #[error("Unexpected name. Expected {0:?} but found {1:?}")]
    UnexpectedName(BoxPrint, BoxPrint),

    #[error("Required box {0:?} was not found.")]
    RequiredBoxNotFound(BoxPrint),

    #[error("Required boxes {0:?} or {1:?} was not found.")]
    RequiredEitherBoxesNotFound(BoxPrint, BoxPrint),

    #[error("Expected at least {1} {0:?} boxes, but found {2}.")]
    NotEnoughBoxes(BoxPrint, u32, u32),

    #[error("Unexpected descriptor tag. Expected 0x{0:x} but found 0x{1:x}")]
    UnexpectedTag(u8, u8),

    #[error("Unsupported MPEG-4 codec {0:02x}")]
    UnsupportedMpeg4Codec(u8),
}

impl From<Mp4BoxError> for AvError {
    fn from(error: Mp4BoxError) -> AvError {
        match error {
            Mp4BoxError::Io(err) => AvError::Io(err),
            _ => AvError::InvalidData,
        }
    }
}

pub(crate) fn non_empty<T>(boxes: Vec<T>, name: BoxName) -> Result<Vec<T>, Mp4BoxError> {
    if !boxes.is_empty() {
        Ok(boxes)
    } else {
        Err(Mp4BoxError::NotEnoughBoxes(BoxPrint(name), 1, 0))
    }
}

pub(crate) fn require_box<T>(val: Option<T>, name: BoxName) -> Result<T, Mp4BoxError> {
    if let Some(val) = val {
        Ok(val)
    } else {
        Err(Mp4BoxError::RequiredBoxNotFound(BoxPrint(name)))
    }
}

pub(crate) fn require_either_box<T>(
    val: Option<T>,
    a: BoxName,
    b: BoxName,
) -> Result<T, Mp4BoxError> {
    if let Some(val) = val {
        Ok(val)
    } else {
        Err(Mp4BoxError::RequiredEitherBoxesNotFound(
            BoxPrint(a),
            BoxPrint(b),
        ))
    }
}

pub(crate) fn goto(buf: &mut dyn Buffered, pos: u64) -> Result<(), Mp4BoxError> {
    buf.seek(SeekFrom::Start(pos))?;

    Ok(())
}

pub(crate) fn pos(buf: &mut dyn Buffered) -> Result<u64, Mp4BoxError> {
    Ok(buf.seek(SeekFrom::Current(0))?)
}

pub(crate) fn skip(buf: &mut dyn Buffered, count: u64) -> Result<(), Mp4BoxError> {
    buf.seek(SeekFrom::Current(count as i64))?;

    Ok(())
}

pub(crate) fn peek(buf: &mut dyn Buffered, size: usize) -> Result<&[u8], Mp4BoxError> {
    if size < buf.data().len() {
        Ok(&buf.data()[..size])
    } else {
        buf.fill_buf()?;

        let data = buf.data();

        if size < data.len() {
            Ok(&data[..size])
        } else {
            Err(Mp4BoxError::UnexpectedEos)
        }
    }
}

impl fmt::Debug for Boks {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{:?} ({})", BoxPrint(self.name), self.size)
    }
}

#[derive(Clone)]
pub struct Boks {
    name: BoxName,
    size: u64,
    read_size: u8,
}

impl Boks {
    pub fn new(name: BoxName) -> Self {
        Boks {
            name,
            size: 0,
            read_size: 0,
        }
    }

    pub fn peek(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        use std::convert::TryInto;

        let contents = peek(buf, 8)?;

        let mut size = BigEndian::read_u32(&contents[0..]) as u64;
        let name = contents[4..].try_into().unwrap();

        if size == 1 {
            let contents = peek(buf, 16)?;
            size = BigEndian::read_u64(&contents[8..]);
        }

        Ok(Boks {
            name,
            size,
            read_size: 0,
        })
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        use std::convert::TryInto;

        let mut contents = [0u8; 8];
        buf.read_exact(&mut contents)?;

        let mut read_size = 8;
        let mut size = BigEndian::read_u32(&contents[0..]) as u64;
        let name = contents[4..].try_into().unwrap();

        if size == 1 {
            buf.read_exact(&mut contents)?;
            size = BigEndian::read_u64(&contents[..]);
            read_size = 16;
        }

        Ok(Boks {
            name,
            size,
            read_size,
        })
    }

    pub fn read_named(buf: &mut dyn Buffered, expected: BoxName) -> Result<Self, Mp4BoxError> {
        use std::convert::TryInto;

        let mut contents = [0u8; 8];
        buf.read_exact(&mut contents)?;

        let mut read_size = 8;
        let mut size = BigEndian::read_u32(&contents[0..]) as u64;
        let name = contents[4..].try_into().unwrap();

        if name != expected {
            return Err(Mp4BoxError::UnexpectedName(
                BoxPrint(expected),
                BoxPrint(name),
            ));
        }

        if size == 1 {
            buf.read_exact(&mut contents)?;
            size = BigEndian::read_u64(&contents[..]);
            read_size = 16;
        }

        Ok(Boks {
            name,
            size,
            read_size,
        })
    }

    fn write(&self, writer: &mut dyn Write, size: u64) -> Result<(), Mp4BoxError> {
        let mut bytes = [0u8; 16];

        if size > u32::MAX as _ {
            BigEndian::write_u32(&mut bytes[..], 1);
            bytes[4..8].copy_from_slice(&self.name);
            BigEndian::write_u64(&mut bytes[8..], size);

            writer.write_all(&bytes[..])?;
        } else {
            BigEndian::write_u32(&mut bytes[..], size as u32);
            bytes[4..8].copy_from_slice(&self.name);

            writer.write_all(&bytes[..8])?;
        }

        Ok(())
    }

    pub fn size(&self, size: u64) -> u64 {
        if size + 8 > u32::MAX as u64 {
            size + 16
        } else {
            size + 8
        }
    }

    pub fn remaining_size(&self) -> u64 {
        self.size - self.read_size as u64
    }
}

#[derive(Debug)]
pub struct FullBox {
    boks: Boks,
    version: u8,
    flags: u32,
    read_size: u8,
}

impl FullBox {
    fn new(name: BoxName, version: u8, flags: u32) -> Self {
        FullBox {
            boks: Boks::new(name),
            version,
            flags,
            read_size: 0,
        }
    }

    fn write(&self, writer: &mut dyn Write, size: u64) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, size)?;

        let mut bytes = [0u8; 4];
        bytes[0] = self.version;
        BigEndian::write_u24(&mut bytes[1..], self.flags);

        writer.write_all(&bytes[..])?;

        Ok(())
    }

    pub fn read_named(buf: &mut dyn Buffered, expected: BoxName) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read_named(buf, expected)?;

        let mut val = [0u8; 4];
        buf.read_exact(&mut val)?;

        let version = val[0];
        let flags = BigEndian::read_u24(&val[1..]);

        Ok(FullBox {
            boks,
            version,
            flags,
            read_size: 4,
        })
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read(buf)?;

        let mut val = [0u8; 4];
        buf.read_exact(&mut val)?;

        let version = val[0];
        let flags = BigEndian::read_u24(&val[1..]);

        Ok(FullBox {
            boks,
            version,
            flags,
            read_size: 4,
        })
    }

    pub fn size(&self, size: u64) -> u64 {
        self.boks.size(size + 4)
    }

    pub fn remaining_size(&self) -> u64 {
        self.boks.size - self.read_size as u64
    }
}

#[derive(Debug)]
pub struct SampleEntry {
    boks: Boks,
    data_reference_index: u16,
}

impl SampleEntry {
    pub fn new(name: BoxName, data_reference_index: u16) -> Self {
        SampleEntry {
            boks: Boks::new(name),
            data_reference_index,
        }
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read(buf)?;

        let mut contents = [0u8; 8];
        buf.read_exact(&mut contents)?;

        let data_reference_index = BigEndian::read_u16(&contents[6..]);

        Ok(SampleEntry {
            boks,
            data_reference_index,
        })
    }

    fn write(&self, writer: &mut dyn Write, size: u64) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, size)?;

        let mut bytes = [0u8; 8];
        BigEndian::write_u16(&mut bytes[6..], self.data_reference_index);

        writer.write_all(&bytes[..])?;

        Ok(())
    }

    fn size(&self, size: u64) -> u64 {
        self.boks.size(size + 8)
    }
}

#[derive(Debug)]
pub struct VisualSampleEntry {
    sample_entry: SampleEntry,
    width: u16,
    height: u16,
    // clap: Option<CleanApertureBox>,
    // pasp: Option<PixelAspectRatioBox>,
}

impl VisualSampleEntry {
    pub fn new(name: BoxName, data_reference_index: u16, width: u16, height: u16) -> Self {
        VisualSampleEntry {
            sample_entry: SampleEntry::new(name, data_reference_index),
            width,
            height,
        }
    }

    pub fn read(buf: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let sample_entry = SampleEntry::read(buf)?;

        let mut contents = [0u8; 70];
        buf.read_exact(&mut contents)?;

        let width = BigEndian::read_u16(&contents[20..]);
        let height = BigEndian::read_u16(&contents[24..]);

        Ok(VisualSampleEntry {
            sample_entry,
            width,
            height,
        })
    }

    fn write(&self, writer: &mut dyn Write, size: u64) -> Result<(), Mp4BoxError> {
        self.sample_entry.write(writer, size)?;

        let mut bytes = [0u8; 70];
        BigEndian::write_u16(&mut bytes[16..], self.width);
        BigEndian::write_u16(&mut bytes[18..], self.height);
        BigEndian::write_u32(&mut bytes[20..], 0x0048_0000);
        BigEndian::write_u32(&mut bytes[24..], 0x0048_0000);
        BigEndian::write_u16(&mut bytes[32..], 1);
        BigEndian::write_u16(&mut bytes[66..], 0x0018);
        BigEndian::write_i16(&mut bytes[68..], -1);

        writer.write_all(&bytes[..])?;

        Ok(())
    }

    fn size(&self, size: u64) -> u64 {
        self.sample_entry.size(size + 70)
    }
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
