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

use av_format::error::Result as AvResult;

use log::*;

use crate::boxes::*;
use crate::{skip, Boks, Mp4BoxError};

use stbl::ChunkOffsets;
use stsz::SampleSizes;

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

    #[rustfmt::skip]
    let data = [
        1, 1, profile,
        2, 1, vpcc.config.level,
        3, 1, bit_depth,
        4, 1, chroma_subsampling,
    ];

    // TODO: missing 12 bit formats
    let mut base = match (profile, bit_depth, chroma_subsampling) {
        (0, 8, 0) => *formats::YUV420,
        (2, 10, 0) => *formats::YUV420_10,

        (1, 8, 0) => *formats::YUV422,
        (3, 10, 0) => *formats::YUV422_10,

        (1, 8, 2) => *formats::YUV444,
        (3, 10, 2) => *formats::YUV444_10,

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
                let width = entry.visual_sample_entry.width as usize;
                let height = entry.visual_sample_entry.height as usize;

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
                let width = entry.visual_sample_entry.width as usize;
                let height = entry.visual_sample_entry.height as usize;

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
    _sample_description_index: u32,
}

fn get_sample_times(stts: stts::TimeToSampleBox) -> Vec<SampleTimes> {
    let mut base = 0;

    let sample_count = stts.entries.len();
    let mut times = Vec::with_capacity(sample_count);

    for entry in stts.entries {
        let count = entry.count;
        let delta = entry.delta;

        times.push(SampleTimes { base, delta, count });

        base += count as u64 * delta as u64;
    }

    times
}

struct Track {
    id: u32,
    index: usize,
    sample_entry: stsd::SampleEntry,
    stsc: Vec<Chunk>,
    chunk_offsets: ChunkOffsets,
    times: Vec<SampleTimes>,
    sizes: SampleSizes,
    sync_samples: Option<Vec<u32>>,

    timebase: Rational64,
    duration: u64,

    // TODO: put all "current" state into own struct?
    current_sync_index: usize,
    current_stsc: usize,
    current_chunk_sample_offset: u64,
    stsc_chunk_index: usize,
    stsc_sample_index: usize,
    current_times: usize,
    time_index: usize,
    current_sample: u64,
}

impl Track {
    fn from_trak(id: u32, trak: trak::TrackBox) -> Self {
        let index = id as usize;

        let sample_entry = trak.mdia.minf.stbl.stsd.entries.into_iter().next().unwrap();
        let timebase = Rational64::new(1, trak.mdia.mdhd.timescale as i64);
        let duration = trak.tkhd.duration;
        let sync_samples = trak.mdia.minf.stbl.stss.map(|s| s.sync_samples);

        let mut chunks: Vec<Chunk> =
            Vec::with_capacity(trak.mdia.minf.stbl.stsc.entries.len() as usize);

        for (idx, entry) in trak.mdia.minf.stbl.stsc.entries.iter().enumerate() {
            let first = entry.first_chunk;

            if idx >= 1 {
                chunks[idx - 1].chunk_count = Some(first);
            }

            chunks.push(Chunk {
                sample_count: entry.samples_per_chunk,
                chunk_count: None,
                _sample_description_index: entry.sample_description_index,
            });
        }

        Track {
            id,
            index,
            sample_entry,
            stsc: chunks,
            chunk_offsets: trak.mdia.minf.stbl.chunk_offsets,
            times: get_sample_times(trak.mdia.minf.stbl.stts),
            sizes: trak.mdia.minf.stbl.stsz.sample_sizes,
            sync_samples,

            timebase,
            duration,

            current_sync_index: 0,
            current_stsc: 0,
            current_chunk_sample_offset: 0,
            stsc_chunk_index: 0,
            stsc_sample_index: 0,
            current_times: 0,
            time_index: 0,
            current_sample: 0,
        }
    }

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
        let keyframe = self
            .sync_samples
            .as_ref()
            .map(|s| s[self.current_sync_index] as u64 == self.current_sample)
            .unwrap_or(true);
        let _chunk = &self.stsc.get(self.current_stsc)?;
        let data_offset =
            self.current_chunk_sample_offset + self.chunk_offsets.get(self.current_stsc)?;

        let times = &self.times.get(self.current_times)?;
        let time = times.base + ((self.time_index as u32) * times.delta) as u64;
        let duration = times.delta;

        let data_length = match &self.sizes {
            SampleSizes::Constant(size) => *size,
            SampleSizes::Variable(ref sizes) => *sizes.get(self.current_sample as usize)?,
        };

        Some(SampleRef {
            time,
            duration,
            data_offset,
            data_length,
            keyframe,
        })
    }

    pub fn advance_sample(&mut self) {
        let chunk = &self.stsc[self.current_stsc];
        let times = &self.times[self.current_times];

        self.current_chunk_sample_offset += match &self.sizes {
            SampleSizes::Constant(size) => *size,
            SampleSizes::Variable(ref sizes) => sizes[self.current_sample as usize],
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

        if let Some(sync_samples) = self.sync_samples.as_ref() {
            if self.current_sample > sync_samples[self.current_sync_index] as u64
                && self.current_sync_index < sync_samples.len() - 1
            {
                self.current_sync_index += 1;
            }
        }
    }
}

pub struct Mp4Demuxer {
    tracks: Vec<Track>,
}

impl Default for Mp4Demuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl Mp4Demuxer {
    pub fn new() -> Self {
        Self { tracks: Vec::new() }
    }

    fn pos(&mut self, buf: &mut dyn Buffered) -> Result<u64, Mp4BoxError> {
        Ok(buf.seek(SeekFrom::Current(0))?)
    }

    fn read_until_moov(&mut self, buf: &mut dyn Buffered) -> Result<(), Mp4BoxError> {
        // go through boxes until we find moov
        loop {
            let pos = self.pos(buf)?;
            let boks = Boks::peek(buf)?;

            debug!("{}: {:?}", pos, boks);

            match &boks.name {
                // b"mdat" => self.mdat_offset = Some(self.offset),
                b"moov" => {
                    let pos = self.pos(buf)?;
                    debug!("found moov box: {}", pos);

                    let moov = moov::MovieBox::read(buf)?;
                    self.tracks = moov
                        .tracks
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| Track::from_trak(i as u32, t))
                        .collect::<Vec<_>>();

                    // self.parse_moov(buf, remaining)?;

                    return Ok(());
                }
                _ => {
                    warn!("skipping box {:?}", boks);
                    skip(buf, boks.size)?;
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
        let res = self.read_until_moov(buf);
        if let Err(e) = res {
            error!("{}", e);
        }

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
