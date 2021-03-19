use av_format::demuxer::Context as DemuxerCtx;
use av_format::muxer::Context as MuxerCtx;
use av_format::muxer::Writer;
use av_format::{buffer::AccReader, demuxer::Event};

use av_mp4::demuxer::Mp4Demuxer;
use av_mp4::muxer::Mp4Muxer;

use log::*;

use std::sync::Arc;

fn main() {
    pretty_env_logger::init();

    let in_path = std::env::args().nth(1).unwrap();
    let out_path = std::env::args().nth(2).unwrap();
    let in_file = std::fs::File::open(&in_path).unwrap();
    let out_file = std::fs::File::create(&out_path).unwrap();

    let acc = AccReader::new(in_file);
    let mut demuxer = DemuxerCtx::new(Box::new(Mp4Demuxer::new()), Box::new(acc));
    let mut muxer = MuxerCtx::new(
        Box::new(Mp4Muxer::new()),
        Writer::from_seekable(Box::new(out_file)),
    );

    debug!("read headers: {:?}", demuxer.read_headers().unwrap());
    debug!("global info: {:#?}", demuxer.info);

    muxer.set_global_info(demuxer.info.clone()).unwrap();
    muxer.write_header().unwrap();

    loop {
        match demuxer.read_event() {
            Ok(event) => match event {
                Event::MoreDataNeeded(sz) => panic!("we needed more data: {} bytes", sz),
                Event::NewStream(s) => panic!("new stream :{:?}", s),
                Event::NewPacket(packet) => {
                    muxer.write_packet(Arc::new(packet)).unwrap();
                }
                Event::Continue => {
                    continue;
                }
                Event::Eof => {
                    debug!("writing trailer");
                    muxer.write_trailer().unwrap();
                    break;
                }
                _ => break,
            },
            Err(e) => {
                error!("error: {:?}", e);
                break;
            }
        }
    }
}
