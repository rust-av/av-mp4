use av_data::{
    packet::Packet,
    params::{CodecParams, MediaKind},
    pixel::Formaton,
    value::Value,
};
use av_format::error::Result as AvResult;
use av_format::{
    common::GlobalInfo,
    muxer::{Muxer, Writer},
    stream::Stream,
};

use byteorder::{BigEndian, WriteBytesExt};

use crate::boxes::*;
use crate::boxes::codec::*;
use crate::{AvError, Boks};

use log::*;

use std::io::{Seek, SeekFrom, Write};
use std::mem;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum Mp4MuxerError {
    #[error("Unsupported codec ID '{0}'")]
    UnsupportedCodec(String),

    #[error("Missing codec ID")]
    MissingCodec,

    #[error("Missing stream info")]
    MissingInfo,

    #[error("Missing codec feature {0}")]
    MissingCodecFeature(u8),
}

impl From<Mp4MuxerError> for AvError {
    fn from(_e: Mp4MuxerError) -> AvError {
        AvError::InvalidData
    }
}

fn get_dimensions_for_codec(params: &CodecParams) -> Option<(usize, usize)> {
    match params.kind.as_ref()? {
        MediaKind::Video(video) => Some((video.width, video.height)),
        _ => None,
    }
}

fn get_formaton_for_codec(params: &CodecParams) -> Option<&Arc<Formaton>> {
    match params.kind.as_ref()? {
        MediaKind::Video(video) => video.format.as_ref(),
        _ => None,
    }
}

struct VpxCodecData {
    profile: u8,
    level: u8,
    bit_depth: u8,
    chroma_subsampling: u8,
    colour_primaries: u8,
    transfer_characteristics: u8,
    matrix_coefficients: u8,
}

// TODO: validate feature value range?
fn parse_vpx_codec_data(format: &Formaton, data: &[u8]) -> Result<VpxCodecData, Mp4MuxerError> {
    let mut i = 0;

    let mut profile = None;
    let mut level = None;
    let mut bit_depth = None;
    let mut chroma_subsampling = None;

    while i < data.len() - 2 {
        let id = data[i];
        let len = data[i + 1];

        match (id, len) {
            (1, 1) => profile = Some(data[i + 2]),
            (2, 1) => level = Some(data[i + 2]),
            (3, 1) => bit_depth = Some(data[i + 2]),
            (4, 1) => chroma_subsampling = Some(data[i + 2]),
            _ => {
                debug!("Skipping codec feature {}", id);
            }
        }

        i += len as usize + 2;
    }

    Ok(VpxCodecData {
        profile: profile.ok_or(Mp4MuxerError::MissingCodecFeature(1))?,
        level: level.ok_or(Mp4MuxerError::MissingCodecFeature(2))?,
        bit_depth: bit_depth.ok_or(Mp4MuxerError::MissingCodecFeature(3))?,
        chroma_subsampling: chroma_subsampling.ok_or(Mp4MuxerError::MissingCodecFeature(4))?,
        colour_primaries: format.primaries as u8,
        transfer_characteristics: format.xfer as u8,
        matrix_coefficients: format.matrix as u8,
    })
}

fn get_sample_entry_for_codec(params: &CodecParams) -> Result<stsd::SampleEntry, Mp4MuxerError> {
    let id = params
        .codec_id
        .as_ref()
        .ok_or(Mp4MuxerError::MissingCodec)?;

    match id.as_str() {
        "vp9" => {
            let (width, height) = get_dimensions_for_codec(params).unwrap();
            let format = get_formaton_for_codec(params).unwrap();
            let extra = params.extradata.clone().unwrap();

            let data = parse_vpx_codec_data(&format, &extra)?;

            let entry = vpxx::Vp9SampleEntryBox::new(
                width as u16,
                height as u16,
                vpcc::VpCodecConfigurationBox::new(vpcc::VpCodecConfigurationRecord {
                    profile: data.profile,
                    level: data.level,
                    bit_depth: data.bit_depth,
                    chroma_subsampling: data.chroma_subsampling,
                    video_full_range_flags: 0, // TODO
                    colour_primaries: data.colour_primaries,
                    transfer_characteristics: data.transfer_characteristics,
                    matrix_coefficients: data.matrix_coefficients,
                }),
            );

            Ok(stsd::SampleEntry::Vp9(entry))
        }
        _ => Err(Mp4MuxerError::UnsupportedCodec(id.clone())),
    }
}

pub struct TrackChunkBuilder {
    stream_index: isize,
    chunks: Vec<stsc::SampleToChunkEntry>,
    times: Vec<stts::TimeToSampleEntry>,
    sync_samples: Vec<u32>,
    sizes: Vec<u32>,
    offsets: Vec<u64>,
    chunk_index: u32,
    sample_index: u32,

    current_chunk: Option<stsc::SampleToChunkEntry>,
    current_time: Option<stts::TimeToSampleEntry>,

    prev_ts: Option<i64>,

    first_packet: bool,
}

impl TrackChunkBuilder {
    pub fn new(stream_index: isize) -> Self {
        Self {
            stream_index,
            chunks: Vec::new(),
            times: Vec::new(),
            sizes: Vec::new(),
            offsets: Vec::new(),
            sync_samples: Vec::new(),
            chunk_index: 1,
            sample_index: 0,

            current_chunk: None,
            current_time: None,

            prev_ts: None,

            first_packet: true,
        }
    }

    fn flush(&mut self) {
        if let Some(chunk) = self.current_chunk {
            self.chunks.push(chunk);
        }

        if let Some(time) = self.current_time {
            self.times.push(time);
        }

        self.current_chunk = None;
        self.current_time = None;
    }

    pub fn into_trak(self, stream: &Stream) -> trak::TrackBox {
        let timebase = (stream.timebase.denom() / stream.timebase.numer()) as u32;

        let (width, height) = get_dimensions_for_codec(&stream.params)
            .map(|(w, h)| (w as u32, h as u32))
            .unwrap_or((0, 0));

        trak::TrackBox::new(
            tkhd::TrackHeaderBox::new(
                tkhd::TrackHeaderFlags::ENABLED | tkhd::TrackHeaderFlags::IN_MOVIE,
                1,
                0,
                width.into(),
                height.into(),
            ),
            mdia::MediaBox::new(
                mdhd::MediaHeaderBox::new(timebase, 0),
                hdlr::HandlerBox::new(0x76696465, String::from("Video Handler")),
                minf::MediaInformationBox::new(
                    minf::MediaHeader::Video(vmhd::VideoMediaHeaderBox::new()),
                    dinf::DataInformationBox::new(dref::DataReferenceBox::new(vec![
                        url::DataEntryUrlBox::new(String::from("")),
                    ])),
                    stbl::SampleTableBox::new(
                        stsd::SampleDescriptionBox::new(vec![get_sample_entry_for_codec(
                            &stream.params,
                        )
                        .unwrap()]),
                        stts::TimeToSampleBox::new(self.times),
                        stsc::SampleToChunkBox::new(self.chunks),
                        stsz::SampleSizeBox::new(stsz::SampleSizes::Variable(self.sizes)),
                        stbl::ChunkOffsets::Co64(co64::ChunkLargeOffsetBox::new(self.offsets)),
                        Some(stss::SyncSampleBox::new(self.sync_samples)),
                    ),
                ),
            ),
        )
    }

    fn take_time_delta(&mut self, packet: &Packet) -> Option<u32> {
        if let Some(duration) = packet.t.duration {
            return Some(duration as u32);
        }

        let ts = match (packet.t.dts, packet.t.pts) {
            (Some(dts), _) => dts,
            (None, Some(pts)) => pts,
            _ => {
                panic!("no time information for packet");
            }
        };

        let delta = self.prev_ts.map(|prev| (ts - prev) as u32);

        self.prev_ts = Some(ts);

        delta
    }

    pub fn add_packet(&mut self, prev_stream: isize, offset: u64, packet: &Packet) {
        let delta = self.take_time_delta(packet).unwrap_or(0);

        if self.current_chunk.is_none() {
            self.offsets.push(offset);
            self.current_chunk = Some(stsc::SampleToChunkEntry {
                first_chunk: self.chunk_index,
                samples_per_chunk: 0,
                sample_description_index: 1,
            });
        }

        if self.current_time.is_none() {
            self.current_time = Some(stts::TimeToSampleEntry { count: 0, delta });
        }

        let current_time = self.current_time.as_mut().unwrap();
        let current_chunk = self.current_chunk.as_mut().unwrap();

        if !self.first_packet && prev_stream != self.stream_index {
            self.chunk_index += 1;

            self.offsets.push(offset);
            self.chunks.push(*current_chunk);

            *current_chunk = stsc::SampleToChunkEntry {
                first_chunk: self.chunk_index,
                samples_per_chunk: 1,
                sample_description_index: 1,
            };
        } else {
            current_chunk.samples_per_chunk += 1;
        }

        if current_time.delta == 0 {
            current_time.delta = delta;
            current_time.count += 1;
        } else if delta == current_time.delta {
            current_time.count += 1;
        } else {
            self.times.push(*current_time);

            *current_time = stts::TimeToSampleEntry { count: 1, delta };
        }

        if packet.is_key {
            self.sync_samples.push(self.sample_index);
        }

        self.sizes.push(packet.data.len() as u32);

        self.sample_index += 1;
        self.first_packet = false;
    }
}

pub struct Mp4Muxer {
    info: Option<GlobalInfo>,
    mdat_start: u64,
    mdat_offset: u64,
    tracks: Vec<TrackChunkBuilder>,
    prev_index: isize,
}

impl Default for Mp4Muxer {
    fn default() -> Self {
        Self::new()
    }
}

impl Mp4Muxer {
    pub fn new() -> Self {
        Self {
            info: None,
            mdat_start: 0,
            mdat_offset: 0,
            tracks: Vec::new(),
            prev_index: 0,
        }
    }

    fn flush(&mut self) {
        for track in &mut self.tracks {
            track.flush();
        }
    }

    pub fn stream_for_index(&self, stream_index: usize) -> Option<&Stream> {
        self.info
            .as_ref()?
            .streams
            .iter()
            .find(|s| s.index == stream_index)
    }

    pub fn take_tracks(&mut self) -> Vec<trak::TrackBox> {
        let mut tracks = Vec::new();
        mem::swap(&mut self.tracks, &mut tracks);

        tracks
            .into_iter()
            .map(|t| {
                let stream = self.stream_for_index(t.stream_index as usize).unwrap();
                t.into_trak(stream)
            })
            .collect::<Vec<_>>()
    }
}

impl Muxer for Mp4Muxer {
    fn set_global_info(&mut self, info: GlobalInfo) -> AvResult<()> {
        self.info = Some(info);

        Ok(())
    }

    fn set_option<'a>(&mut self, _key: &str, _val: Value<'a>) -> AvResult<()> {
        Ok(())
    }

    fn configure(&mut self) -> AvResult<()> {
        Ok(())
    }

    fn write_header(&mut self, out: &mut Writer) -> AvResult<()> {
        // TODO: what to pick
        let brands = [*b"iso5"];
        let ftyp = ftyp::FileTypeBox::new(*b"isom", 0, (&brands[..]).into());

        let offset = ftyp.total_size();

        debug!("offset: {}", offset);
        ftyp.write(out)?;

        self.mdat_start = offset;
        self.mdat_offset = offset + 16;

        Boks::new(*b"mdat").write(out, u64::MAX as u64)?;

        Ok(())
    }

    fn write_packet(&mut self, out: &mut Writer, packet: Arc<Packet>) -> AvResult<()> {
        let offset = self.mdat_offset;
        out.write_all(&packet.data)?;
        self.mdat_offset += packet.data.len() as u64;

        if let Some(builder) = self
            .tracks
            .iter_mut()
            .find(|t| t.stream_index == packet.stream_index)
        {
            builder.add_packet(self.prev_index, offset, &packet);
        } else {
            let mut builder = TrackChunkBuilder::new(packet.stream_index);
            builder.add_packet(self.prev_index, offset, &packet);

            self.tracks.push(builder);
        }

        self.prev_index = packet.stream_index;

        Ok(())
    }

    fn write_trailer(&mut self, out: &mut Writer) -> AvResult<()> {
        self.flush();

        let info = self.info.as_ref().ok_or(Mp4MuxerError::MissingInfo)?;
        let timebase = info
            .timebase
            .map(|t| (t.denom() / t.numer()) as u32)
            .unwrap_or(10_000);

        let moov = moov::MovieBox::new(
            mvhd::MovieHeaderBox::new(timebase, 0),
            None,
            self.take_tracks(),
        );

        moov.write(out)?;

        debug!(
            "Seeking to {:08x}, len: {}",
            self.mdat_start + 8,
            self.mdat_offset
        );
        out.seek(SeekFrom::Start(self.mdat_start + 8))?;
        out.write_u64::<BigEndian>(self.mdat_offset - self.mdat_start)?;

        Ok(())
    }
}
