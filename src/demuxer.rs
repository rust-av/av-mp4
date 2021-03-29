use std::io::SeekFrom;

use av_data::{
    packet::Packet,
    params::{CodecParams, MediaKind, VideoInfo},
    pixel::{ColorPrimaries, Formaton, FromPrimitive, MatrixCoefficients, TransferCharacteristic},
    timeinfo::TimeInfo,
};
use av_format::{
    buffer::Buffered,
    common::GlobalInfo,
    demuxer::{Demuxer, Descr, Descriptor, Event},
    rational::Rational64,
    stream::Stream,
};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};

use av_format::error::Error as AvError;
use av_format::error::Result as AvResult;

use log::*;

use crate::boxes::*;
use crate::{read_box_flags, read_box_header, BoxPrint};

use std::sync::Arc;

struct VpxCodecData {
    format: Formaton,
    extradata: Vec<u8>,
}

fn get_vpx_codec_data(vpcc: &vpcc::VpCodecConfigurationBox) -> VpxCodecData {
    use av_data::pixel::formats;

    let profile = vpcc.config.profile;
    let bit_depth = vpcc.config.bit_depth;
    let chroma_subsampling = vpcc.config.chroma_subsampling;

    let data = [
        1, 1, profile,
        2, 1, vpcc.config.level,
        3, 1, bit_depth,
        4, 1, chroma_subsampling,
    ];

    // TODO: missing 12 bit formats
    let mut base = match (profile, bit_depth, chroma_subsampling) {
        (0, 8, 0) => formats::YUV420.clone(),
        (2, 10, 0) => formats::YUV420_10.clone(),

        (1, 8, 0) => formats::YUV422.clone(),
        (3, 10, 0) => formats::YUV422_10.clone(),

        (1, 8, 2) => formats::YUV444.clone(),
        (3, 10, 2) => formats::YUV444_10.clone(),

        _ => panic!(
            "unknown chroma subsampling: {} {} {}",
            profile, bit_depth, chroma_subsampling
        ), // TODO: error
    };

    // TODO: error
    base.primaries = ColorPrimaries::from_u8(vpcc.config.colour_primaries).unwrap();
    base.xfer = TransferCharacteristic::from_u8(vpcc.config.transfer_characteristics).unwrap();
    base.matrix = MatrixCoefficients::from_u8(vpcc.config.matrix_coefficients).unwrap();

    VpxCodecData {
        format: base,
        extradata: data.to_vec(),
    }
}

impl stsd::SampleEntry {
    fn as_codec_params(&self) -> CodecParams {
        match self {
            stsd::SampleEntry::Vp9(entry) => {
                let width = entry.width as usize;
                let height = entry.height as usize;

                let codec_data = get_vpx_codec_data(&entry.vpcc);

                CodecParams {
                    kind: Some(MediaKind::Video(VideoInfo {
                        width,
                        height,
                        format: Some(Arc::new(codec_data.format)),
                    })),
                    codec_id: Some("vp9".into()),
                    extradata: Some(codec_data.extradata),
                    bit_rate: 0,
                    convergence_window: 0,
                    delay: 0,
                }
            }
            stsd::SampleEntry::Avc(entry) => {
                let width = entry.width as usize;
                let height = entry.height as usize;

                let mut parameter_sets = vec![0, 0, 1];
                parameter_sets.extend(&entry.avcc.config.sequence_parameter_sets[0].0);
                if let Some(pps) = entry.avcc.config.picture_parameter_sets.get(0) {
                    parameter_sets.extend(&[0, 0, 1][..]);
                    parameter_sets.extend(&pps.0);
                }

                CodecParams {
                    kind: Some(MediaKind::Video(VideoInfo {
                        width,
                        height,
                        format: None,
                    })),
                    codec_id: Some("h264".into()),
                    extradata: Some(parameter_sets),
                    bit_rate: 0,
                    convergence_window: 0,
                    delay: 0,
                }
            }
        }
    }
}

fn parse_stco(buf: &mut dyn Buffered) -> AvResult<u32> {
    Ok(buf.read_u32::<BigEndian>()?)
}

struct ChunkOffsetIterator<'a> {
    buf: &'a mut dyn Buffered,
    len: u32,
    idx: u32,
}

impl<'a> ChunkOffsetIterator<'a> {
    pub fn new(buf: &'a mut dyn Buffered, len: u32) -> Self {
        Self { buf, len, idx: 0 }
    }

    pub fn len(&self) -> u32 {
        self.len
    }
}

impl<'a> Iterator for ChunkOffsetIterator<'a> {
    type Item = AvResult<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            None
        } else {
            self.idx += 1;

            let result = parse_stco(self.buf);

            Some(result)
        }
    }
}

fn parse_stsc(buf: &mut dyn Buffered) -> AvResult<SampleToChunkEntry> {
    let mut data = [0u8; 12];

    buf.read_exact(&mut data)?;

    let first_chunk = BigEndian::read_u32(&data);
    let samples_per_chunk = BigEndian::read_u32(&data[4..]);
    let sample_description_index = BigEndian::read_u32(&data[8..]);

    Ok(SampleToChunkEntry {
        first_chunk,
        samples_per_chunk,
        sample_description_index,
    })
}

struct SampleToChunkEntryIterator<'a> {
    buf: &'a mut dyn Buffered,
    len: u32,
    idx: u32,
}

impl<'a> SampleToChunkEntryIterator<'a> {
    pub fn new(buf: &'a mut dyn Buffered, len: u32) -> Self {
        Self { buf, len, idx: 0 }
    }

    pub fn len(&self) -> u32 {
        self.len
    }
}

impl<'a> Iterator for SampleToChunkEntryIterator<'a> {
    type Item = AvResult<SampleToChunkEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.len {
            None
        } else {
            self.idx += 1;

            let result = parse_stsc(self.buf);

            Some(result)
        }
    }
}

#[derive(Debug)]
pub struct SampleToChunkEntry {
    first_chunk: u32,
    samples_per_chunk: u32,
    sample_description_index: u32,
}

// "get all samples in next chunk"
// "get chunk at time T"

struct SampleRef {
    time: u64,
    duration: u32,
    data_offset: u64,
    data_length: u32,
    keyframe: bool,
}

struct Sample {
    time: u64,
    duration: u32,
    data: Vec<u8>,
    keyframe: bool,
}

// range = base..(delta*count)
struct SampleTimes {
    base: u64,
    delta: u32,
    count: u32,
}

// range = sample_offset..sample_count
struct Chunk {
    sample_count: u32,
    chunk_count: Option<u32>,
    sample_description_index: u32,
}

enum SampleSize {
    Constant(u32),
    Variable(Vec<u32>),
}

struct Track {
    id: u32,
    index: usize,
    sample_entry: stsd::SampleEntry,
    stsc: Vec<Chunk>,
    chunk_offsets: ChunkOffsets,
    times: Vec<SampleTimes>,
    sizes: SampleSize,
    sync_samples: Vec<u32>,

    timebase: Rational64,
    duration: u64,

    // TODO: put all "current" state into own struct?
    current_sync_index: usize,
    current_chunk: usize,
    current_stsc: usize,
    current_chunk_sample_offset: u64,
    stsc_chunk_index: usize,
    stsc_sample_index: usize,
    current_times: usize,
    time_index: usize,
    current_sample: u64,
}

impl Track {
    pub fn as_stream(&self) -> Stream {
        Stream {
            id: self.id as isize,
            index: self.index,
            params: self.sample_entry.as_codec_params(),
            start: None,
            duration: Some(self.duration),
            timebase: self.timebase,
            user_private: None,
        }
    }

    pub fn current_sample(&self) -> Option<SampleRef> {
        let keyframe = self.sync_samples[self.current_sync_index] as u64 == self.current_sample;
        let _chunk = &self.stsc.get(self.current_stsc)?;
        let data_offset =
            self.current_chunk_sample_offset + self.chunk_offsets.get(self.current_stsc)?;

        let times = &self.times.get(self.current_times)?;
        let time = times.base + ((self.time_index as u32) * times.delta) as u64;
        let duration = times.delta;

        let data_length = match &self.sizes {
            &SampleSize::Constant(size) => size,
            &SampleSize::Variable(ref sizes) => *sizes.get(self.current_sample as usize)?,
        };

        Some(SampleRef {
            time,
            duration,
            data_offset,
            data_length,
            keyframe,
        })
    }

    pub fn seek_to_time(&mut self, _time: u64) {
        todo!()
    }

    pub fn advance_sample(&mut self) {
        // println!("current_sample={},sync_index={},sync={:?}", self.current_sample, self.current_sync_index, self.sync_samples);
        let chunk = &self.stsc[self.current_stsc];
        let times = &self.times[self.current_times];

        self.current_chunk_sample_offset += match &self.sizes {
            &SampleSize::Constant(size) => size,
            &SampleSize::Variable(ref sizes) => sizes[self.current_sample as usize],
        } as u64;

        self.time_index += 1;
        if self.time_index >= times.count as usize {
            self.time_index = 0;
            self.current_times += 1;
        }

        self.stsc_sample_index += 1;

        // if we are past samples for a single chunk in a stsc entry, move on
        // to next chunk
        if self.stsc_sample_index >= chunk.sample_count as usize {
            self.stsc_sample_index = 0;
            self.stsc_chunk_index += 1;

            // if we are past chunks in the stsc entry, move to next entry
            if let Some(chunk_count) = chunk.chunk_count {
                if self.stsc_chunk_index >= chunk_count as usize {
                    self.current_stsc += 1;
                    self.current_chunk_sample_offset = 0;
                }
            }
        }

        // always advance one sample
        self.current_sample += 1;

        if self.current_sample > self.sync_samples[self.current_sync_index] as u64 && self.current_sync_index < self.sync_samples.len() - 1 {
            self.current_sync_index += 1;
        }
    }
}

enum ChunkOffsets {
    Stco(Vec<u32>),
    Co64(Vec<u64>),
}

impl ChunkOffsets {
    fn get(&self, index: usize) -> Option<u64> {
        match self {
            ChunkOffsets::Stco(offsets) => offsets.get(index).map(|o| *o as u64),
            ChunkOffsets::Co64(offsets) => offsets.get(index).map(|o| *o),
        }
    }
}

#[derive(Default)]
struct TrackDemuxer {
    tkhd: Option<tkhd::TrackHeaderBox>,
    mdhd: Option<mdhd::MediaHeaderBox>,
    sample_entry: Option<stsd::SampleEntry>,
    chunks: Option<Vec<Chunk>>,
    chunk_offsets: Option<ChunkOffsets>,
    sync_samples: Option<Vec<u32>>,
    sizes: Option<SampleSize>,
    times: Option<Vec<SampleTimes>>,
    //stsd: Option<stsd::SampleDescriptionBox>
}

fn require<T>(val: Option<T>, msg: &'static str) -> AvResult<T> {
    if let Some(val) = val {
        Ok(val)
    } else {
        error!("{}", msg);

        Err(AvError::InvalidData)
    }
}

impl TrackDemuxer {
    pub fn new() -> Self {
        TrackDemuxer::default()
    }

    fn build(self, index: usize) -> AvResult<Track> {
        let tkhd = require(self.tkhd, "missing tkhd")?;
        let mdhd = require(self.mdhd, "missing mdhd")?;
        let sample_entry = require(self.sample_entry, "missing stsd entry")?;
        let chunk_offsets = require(self.chunk_offsets, "missing stco/co64 entry")?;
        let times = require(self.times, "missing stts entry")?;
        let sizes = require(self.sizes, "missing stsz entry")?;
        let chunks = require(self.chunks, "missing stsc entry")?;
        let sync_samples = require(self.sync_samples, "missing stss entry")?; // TODO: not required

        Ok(Track {
            id: tkhd.track_id,
            index,
            sample_entry,
            stsc: chunks,
            chunk_offsets,
            times,
            sizes,
            sync_samples,

            timebase: Rational64::new(1, mdhd.timescale as i64),
            duration: tkhd.duration,

            current_chunk: 0,
            current_chunk_sample_offset: 0,
            current_stsc: 0,
            stsc_chunk_index: 0,
            stsc_sample_index: 0,
            current_times: 0,
            time_index: 0,
            current_sample: 0,
            current_sync_index: 0,
        })
    }

    fn on_tkhd(&mut self, tkhd: tkhd::TrackHeaderBox) -> AvResult<()> {
        self.tkhd = Some(tkhd);
        Ok(())
    }

    fn on_mdhd(&mut self, mdhd: mdhd::MediaHeaderBox) -> AvResult<()> {
        self.mdhd = Some(mdhd);
        Ok(())
    }

    fn on_stsc(&mut self, iter: SampleToChunkEntryIterator) -> AvResult<()> {
        let mut chunks: Vec<Chunk> = Vec::with_capacity(iter.len() as usize);

        for (idx, entry) in iter.enumerate() {
            let entry = entry?;

            let first = entry.first_chunk;

            if idx >= 1 {
                chunks[idx - 1].chunk_count = Some(first);
            }

            chunks.push(Chunk {
                sample_count: entry.samples_per_chunk,
                chunk_count: None,
                sample_description_index: entry.sample_description_index,
            });
        }

        self.chunks = Some(chunks);

        Ok(())
    }

    fn on_time_to_samples(&mut self, times: Vec<SampleTimes>) -> AvResult<()> {
        self.times = Some(times);
        Ok(())
    }

    fn on_sync_samples(&mut self, samples: Vec<u32>) -> AvResult<()> {
        self.sync_samples = Some(samples);
        Ok(())
    }

    fn on_chunk_offsets(&mut self, offsets: ChunkOffsets) -> AvResult<()> {
        self.chunk_offsets = Some(offsets);
        Ok(())
    }

    fn on_sample_size(&mut self, sample_size: SampleSize) -> AvResult<()> {
        self.sizes = Some(sample_size);
        Ok(())
    }

    fn on_sample_entry(&mut self, entry: stsd::SampleEntry) -> AvResult<()> {
        self.sample_entry = Some(entry);
        Ok(())
    }
}

pub struct Mp4Demuxer {
    tracks: Vec<Track>,
}

impl Mp4Demuxer {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }

    fn skip(&mut self, buf: &mut dyn Buffered, count: i64) -> AvResult<()> {
        debug!("skipping {} bytes", count);

        buf.seek(SeekFrom::Current(count))?;

        Ok(())
    }

    fn pos(&mut self, buf: &mut dyn Buffered) -> AvResult<u64> {
        Ok(buf.seek(SeekFrom::Current(0))?)
    }

    fn goto(&mut self, buf: &mut dyn Buffered, pos: u64) -> AvResult<()> {
        buf.seek(SeekFrom::Start(pos))?;

        Ok(())
    }

    fn parse_stss(
        &mut self,
        buf: &mut dyn Buffered,
        _size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let (_version, _flags) = read_box_flags(buf)?;
        let sample_count = buf.read_u32::<BigEndian>()?;

        debug!("stss entry count: {}", sample_count);

        let mut sync_samples = Vec::with_capacity(sample_count as usize);

        for _i in 0..sample_count {
            let sample_index = buf.read_u32::<BigEndian>()?;

            sync_samples.push(sample_index);
        }

        track_demuxer.on_sync_samples(sync_samples)?;

        Ok(())
    }

    fn parse_stts(
        &mut self,
        buf: &mut dyn Buffered,
        _size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let (_version, _flags) = read_box_flags(buf)?;
        let sample_count = buf.read_u32::<BigEndian>()?;

        debug!("stts sample count: {}", sample_count);

        let mut data = [0u8; 8];
        let mut base = 0;

        let mut times = Vec::with_capacity(sample_count as usize);

        for _i in 0..sample_count {
            buf.read_exact(&mut data)?;

            let count = BigEndian::read_u32(&data);
            let delta = BigEndian::read_u32(&data[4..]);

            times.push(SampleTimes { base, delta, count });

            base += count as u64 * delta as u64;
        }

        track_demuxer.on_time_to_samples(times)?;

        Ok(())
    }

    fn parse_stsc(
        &mut self,
        buf: &mut dyn Buffered,
        _size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let (_version, _flags) = read_box_flags(buf)?;
        let entry_count = buf.read_u32::<BigEndian>()?;

        debug!("stsc entry count: {}", entry_count);

        let iter = SampleToChunkEntryIterator::new(buf, entry_count);

        track_demuxer.on_stsc(iter)?;

        /*let mut data = [0u8; 12];

        for i in 0..entry_count {
            buf.read_exact(&mut data)?;

            let first_chunk = BigEndian::read_u32(&data);
            let samples_per_chunk = BigEndian::read_u32(&data[4..]);
            let sample_description_index = BigEndian::read_u32(&data[8..]);

            track_demuxer.on_sample_to_chunk_entry(SampleToChunkEntry {
                first_chunk,
                samples_per_chunk,
                sample_description_index,
            })?;
        }

        track_demuxer.on_time_to_sample_end()?;*/

        Ok(())
    }

    fn parse_stsz(
        &mut self,
        buf: &mut dyn Buffered,
        _size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let (_version, _flags) = read_box_flags(buf)?;
        let sample_size = buf.read_u32::<BigEndian>()?;
        let sample_count = buf.read_u32::<BigEndian>()?;

        if sample_size != 0 {
            track_demuxer.on_sample_size(SampleSize::Constant(sample_size))?;
        } else {
            let mut sizes = Vec::with_capacity(sample_count as usize);

            for _i in 0..sample_count {
                let sample_size = buf.read_u32::<BigEndian>()?;

                sizes.push(sample_size);
            }

            track_demuxer.on_sample_size(SampleSize::Variable(sizes))?;
        }

        Ok(())
    }

    fn parse_co64(
        &mut self,
        buf: &mut dyn Buffered,
        _size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let (_version, _flags) = read_box_flags(buf)?;

        let entry_count = buf.read_u32::<BigEndian>()?;

        debug!("co64 entry count: {}", entry_count);

        let mut offsets = Vec::with_capacity(entry_count as usize);

        for _i in 0..entry_count {
            let chunk_offset = buf.read_u64::<BigEndian>()?;

            offsets.push(chunk_offset as u64);
        }

        track_demuxer.on_chunk_offsets(ChunkOffsets::Co64(offsets))?;

        Ok(())
    }

    fn parse_stco(
        &mut self,
        buf: &mut dyn Buffered,
        _size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let (_version, _flags) = read_box_flags(buf)?;

        let entry_count = buf.read_u32::<BigEndian>()?;

        debug!("stco entry count: {}", entry_count);

        let mut offsets = Vec::with_capacity(entry_count as usize);

        for _i in 0..entry_count {
            let chunk_offset = buf.read_u32::<BigEndian>()?;

            offsets.push(chunk_offset as u64);
        }

        track_demuxer.on_chunk_offsets(ChunkOffsets::Co64(offsets))?;

        Ok(())
    }

    fn parse_stsd(
        &mut self,
        buf: &mut dyn Buffered,
        size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let (_version, _flags) = read_box_flags(buf)?;

        let sample_count = buf.read_u32::<BigEndian>()?;

        debug!("stsd sample count: {}", sample_count);

        for _ in 0..sample_count {
            let pos = self.pos(buf)?;
            let (name, total, remaining) = read_box_header(buf)?;
            debug!("{:?} {} {}", BoxPrint(name), pos, total);

            match &name {
                b"avc1" => {
                    track_demuxer.on_sample_entry(stsd::SampleEntry::Avc(
                        avc1::AvcSampleEntryBox::read(buf).unwrap(),
                    ))?;
                }
                b"vp09" => {
                    track_demuxer.on_sample_entry(stsd::SampleEntry::Vp9(
                        vpxx::Vp9SampleEntryBox::read(buf).unwrap(),
                    ))?;
                }
                _ => {
                    warn!("skipping stsd sample entry {:?}", BoxPrint(name));

                    self.skip(buf, remaining as i64)?;
                }
            }
        }

        Ok(())
    }

    fn parse_stbl(
        &mut self,
        buf: &mut dyn Buffered,
        size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let end = self.pos(buf)? + size;

        loop {
            let pos = self.pos(buf)?;

            if pos >= end {
                break;
            }

            let (name, total_size, remaining) = read_box_header(buf)?;

            debug!("{:?} {} {}", BoxPrint(name), pos, total_size);
            let _end = pos + total_size;

            match &name {
                b"stsd" => self.parse_stsd(buf, remaining, track_demuxer)?,
                b"stts" => self.parse_stts(buf, remaining, track_demuxer)?,
                b"stss" => self.parse_stss(buf, remaining, track_demuxer)?,
                b"stsc" => self.parse_stsc(buf, remaining, track_demuxer)?,
                b"stsz" => self.parse_stsz(buf, remaining, track_demuxer)?,
                b"stco" => self.parse_stco(buf, remaining, track_demuxer)?,
                b"co64" => self.parse_co64(buf, remaining, track_demuxer)?,
                _ => {
                    warn!("skipping stbl box {:?} ({})", BoxPrint(name), remaining);

                    self.skip(buf, remaining as i64)?;
                }
            }
        }

        Ok(())
    }

    fn parse_mdia(
        &mut self,
        buf: &mut dyn Buffered,
        size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let end = self.pos(buf)? + size;

        loop {
            let pos = self.pos(buf)?;

            if pos >= end {
                break;
            }

            let (name, total, remaining) = read_box_header(buf)?;
            let _end = pos + total;
            debug!("{:?} {} {}", BoxPrint(name), pos, total);

            match &name {
                b"mdhd" => {
                    let mdhd = mdhd::MediaHeaderBox::read(buf).unwrap();

                    debug!("Parsed mdhd: {:#?}", mdhd);

                    track_demuxer.on_mdhd(mdhd)?;
                }
                b"minf" => self.parse_minf(buf, remaining, track_demuxer)?,
                _ => {
                    warn!("skipping mdia box {:?}", BoxPrint(name));

                    self.skip(buf, remaining as i64)?;
                }
            }
        }

        Ok(())
    }

    fn parse_minf(
        &mut self,
        buf: &mut dyn Buffered,
        size: u64,
        track_demuxer: &mut TrackDemuxer,
    ) -> AvResult<()> {
        let end = self.pos(buf)? + size;

        loop {
            let pos = self.pos(buf)?;

            if pos >= end {
                break;
            }

            let (name, total, remaining) = read_box_header(buf)?;
            let _end = pos + total;
            debug!("{:?} {} {}", BoxPrint(name), pos, total);

            match &name {
                b"stbl" => self.parse_stbl(buf, remaining, track_demuxer)?,
                _ => {
                    warn!("skipping minf box {:?}", BoxPrint(name));

                    self.skip(buf, remaining as i64)?;
                }
            }
        }

        Ok(())
    }

    fn parse_trak(
        &mut self,
        buf: &mut dyn Buffered,
        size: u64,
        index: usize,
    ) -> AvResult<Track> {
        let mut track_demuxer = TrackDemuxer::new();

        let end = self.pos(buf)? + size;

        loop {
            let pos = self.pos(buf)?;

            if pos >= end {
                break;
            }

            let (name, total, remaining) = read_box_header(buf)?;
            let _end = pos + total;
            debug!("{:?} {} {}", BoxPrint(name), pos, total);

            match &name {
                b"tkhd" => {
                    let tkhd = tkhd::TrackHeaderBox::read(buf).unwrap();

                    debug!("Parsed tkhd: {:#?}", tkhd);

                    track_demuxer.on_tkhd(tkhd)?;
                }
                // b"mvex" => {},
                b"mdia" => self.parse_mdia(buf, remaining, &mut track_demuxer)?,
                _ => {
                    warn!("skipping trak box {:?}", BoxPrint(name));

                    self.skip(buf, remaining as i64)?;
                }
            }
        }

        track_demuxer.build(index)
    }

    fn parse_moov(&mut self, buf: &mut dyn Buffered, size: u64) -> AvResult<()> {
        let end = self.pos(buf)? + size;

        loop {
            let pos = self.pos(buf)?;

            if pos >= end {
                break;
            }

            let (name, total, remaining) = read_box_header(buf)?;
            let end = pos + total;
            debug!("{:?} {} {}", BoxPrint(name), pos, total);

            match &name {
                // b"mvhd" => {},
                // b"mvex" => {},
                b"trak" => {
                    if let Ok(track) = self.parse_trak(buf, remaining, self.tracks.len()) {
                        self.tracks.push(track);
                    } else {
                        debug!("seeking to {} to skip invalid trak", end);

                        // TODO: wrong end because doesnt include whole box
                        //       size, only contents
                        self.goto(buf, end)?;
                    }
                }
                _ => {
                    warn!("skipping moov entry {:?}", BoxPrint(name));

                    self.skip(buf, remaining as i64)?;
                }
            }
        }

        Ok(())
    }

    fn read_until_moov(&mut self, buf: &mut dyn Buffered) -> AvResult<()> {
        // go through boxes until we find moov
        loop {
            let pos = self.pos(buf)?;
            let (name, total, remaining) = read_box_header(buf).unwrap();
            debug!("{:?} {} {}", BoxPrint(name), pos, total);

            match &name {
                // b"mdat" => self.mdat_offset = Some(self.offset),
                b"moov" => {
                    debug!("found moov box");

                    self.parse_moov(buf, remaining)?;

                    return Ok(());
                }
                _ => {
                    warn!("skipping mp4 box {:?}", BoxPrint(name));

                    debug!(
                        "pos before {}, {}",
                        self.pos(buf)?,
                        self.pos(buf)? + remaining
                    );
                    self.skip(buf, remaining as i64)?;
                    debug!("pos after {}", self.pos(buf)?);
                }
            }
        }
    }

    fn read_sample(&mut self, buf: &mut dyn Buffered, sample: SampleRef) -> AvResult<Sample> {
        buf.seek(SeekFrom::Start(sample.data_offset))?;
        let mut data = vec![0u8; sample.data_length as usize];
        buf.read_exact(&mut data)?;

        Ok(Sample {
            time: sample.time,
            duration: sample.duration,
            data,
            keyframe: sample.keyframe,
        })
    }

    fn read_next_event(&mut self, buf: &mut dyn Buffered) -> AvResult<Event> {
        let earliest_track_sample = self
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(idx, t)| t.current_sample().map(|s| (idx, s)))
            .min_by_key(|(_idx, s)| s.time);

        if let Some((track, sample)) = earliest_track_sample {
            let sample = self.read_sample(buf, sample)?;

            let track = &mut self.tracks[track];
            track.advance_sample();

            let time = TimeInfo {
                pts: Some(sample.time as i64),
                dts: Some(sample.time as i64),
                duration: Some(sample.duration as u64),
                timebase: Some(track.timebase),
                user_private: None,
            };

            let packet = Packet {
                data: sample.data,
                pos: None,
                stream_index: track.index as isize,
                t: time,
                is_key: sample.keyframe,
                is_corrupted: false,
            };

            Ok(Event::NewPacket(packet))
        } else {
            Ok(Event::Eof)
        }
    }
}

impl Demuxer for Mp4Demuxer {
    fn read_headers(
        &mut self,
        buf: &mut dyn Buffered,
        info: &mut GlobalInfo,
    ) -> AvResult<SeekFrom> {
        self.read_until_moov(buf)?;

        info.streams = self.tracks.iter().map(|t| t.as_stream()).collect();

        Ok(SeekFrom::Current(0))
    }

    fn read_event(&mut self, buf: &mut dyn Buffered) -> AvResult<(SeekFrom, Event)> {
        let event = self.read_next_event(buf)?;

        Ok((SeekFrom::Current(0), event))
    }
}

struct Des {
    d: Descr,
}

impl Descriptor for Des {
    fn create(&self) -> Box<dyn Demuxer> {
        Box::new(Mp4Demuxer::new())
    }
    fn describe(&self) -> &Descr {
        &self.d
    }
    fn probe(&self, _data: &[u8]) -> u8 {
        0
    }
}

pub const MP4_DESC: &dyn Descriptor = &Des {
    d: Descr {
        name: "mp4",
        demuxer: "mp4",
        description: "MP4 demuxer",
        extensions: &["mp4"],
        mime: &["video/mp4", "audio/mp4"],
    },
};
