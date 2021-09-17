use crate::*;

use super::{
    co64::ChunkLargeOffsetBox, stco::ChunkOffsetBox, stsc::SampleToChunkBox,
    codec::stsd::SampleDescriptionBox, stss::SyncSampleBox, stsz::SampleSizeBox, stts::TimeToSampleBox,
};

use std::io::Write;

pub enum ChunkOffsets {
    Co64(ChunkLargeOffsetBox),
    Stco(ChunkOffsetBox),
}

impl ChunkOffsets {
    pub fn get(&self, index: usize) -> Option<u64> {
        match self {
            ChunkOffsets::Stco(offsets) => offsets.chunk_offsets.get(index).map(|o| *o as u64),
            ChunkOffsets::Co64(offsets) => offsets.chunk_offsets.get(index).copied(),
        }
    }

    fn size(&self) -> u64 {
        match self {
            ChunkOffsets::Co64(co64) => co64.total_size(),
            ChunkOffsets::Stco(stco) => stco.total_size(),
        }
    }
}

pub struct SampleTableBox {
    boks: Boks,
    pub stsd: SampleDescriptionBox,
    pub stts: TimeToSampleBox,
    pub stsc: SampleToChunkBox,
    pub stsz: SampleSizeBox,
    pub chunk_offsets: ChunkOffsets,
    pub stss: Option<SyncSampleBox>,
}

impl SampleTableBox {
    pub fn new(
        stsd: SampleDescriptionBox,
        stts: TimeToSampleBox,
        stsc: SampleToChunkBox,
        stsz: SampleSizeBox,
        chunk_offsets: ChunkOffsets,
        stss: Option<SyncSampleBox>,
    ) -> Self {
        SampleTableBox {
            boks: Boks::new(*b"stbl"),
            stsd,
            stts,
            stsc,
            stsz,
            chunk_offsets,
            stss,
        }
    }

    pub fn read(reader: &mut dyn Buffered) -> Result<Self, Mp4BoxError> {
        let boks = Boks::read_named(reader, *b"stbl")?;

        let mut stsd = None;
        let mut stts = None;
        let mut stsc = None;
        let mut stsz = None;
        let mut chunk_offsets = None;
        let mut stss = None;

        let iter = BoksIterator::new(reader, boks.remaining_size());
        while let Some((pos, boks)) = iter.next(reader) {
            debug!("{}: {:?}", pos, boks);

            match &boks.name {
                b"stsd" => stsd = Some(SampleDescriptionBox::read(reader)?),
                b"stts" => stts = Some(TimeToSampleBox::read(reader)?),
                b"stsc" => stsc = Some(SampleToChunkBox::read(reader)?),
                b"stsz" => stsz = Some(SampleSizeBox::read(reader)?),
                b"co64" => {
                    chunk_offsets = Some(ChunkOffsets::Co64(ChunkLargeOffsetBox::read(reader)?))
                }
                b"stco" => chunk_offsets = Some(ChunkOffsets::Stco(ChunkOffsetBox::read(reader)?)),
                b"stss" => stss = Some(SyncSampleBox::read(reader)?),
                _ => {
                    warn!("skipping stbl box {:?}", boks);
                    skip(reader, boks.size)?;
                }
            }
        }

        Ok(SampleTableBox {
            boks,
            stsd: require_box(stsd, *b"stsd")?,
            stts: require_box(stts, *b"stts")?,
            stsc: require_box(stsc, *b"stsc")?,
            stsz: require_box(stsz, *b"stsz")?,
            chunk_offsets: require_either_box(chunk_offsets, *b"co64", *b"stco")?,
            stss,
        })
    }

    pub fn write(self, writer: &mut dyn Write) -> Result<(), Mp4BoxError> {
        self.boks.write(writer, self.total_size())?;

        self.stsd.write(writer)?;
        self.stts.write(writer)?;
        self.stsc.write(writer)?;
        self.stsz.write(writer)?;
        match self.chunk_offsets {
            ChunkOffsets::Co64(co64) => co64.write(writer)?,
            ChunkOffsets::Stco(stco) => stco.write(writer)?,
        }
        if let Some(stss) = self.stss {
            stss.write(writer)?;
        }

        Ok(())
    }

    pub fn total_size(&self) -> u64 {
        self.boks.size(self.size())
    }

    fn size(&self) -> u64 {
        self.stsd.total_size()
            + self.stts.total_size()
            + self.stsc.total_size()
            + self.stsz.total_size()
            + self.chunk_offsets.size()
            + self.stss.as_ref().map(|b| b.total_size()).unwrap_or(0)
    }
}
