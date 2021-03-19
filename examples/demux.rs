use av_format::demuxer::Context as DemuxerCtx;
use av_format::{buffer::AccReader, demuxer::Event};

use av_mp4::demuxer::Mp4Demuxer;

use log::*;

fn main() {
    pretty_env_logger::init();

    let path = std::env::args().nth(1).unwrap();
    let file = std::fs::File::open(&path).unwrap();

    let acc = AccReader::new(file);
    let mut demuxer = DemuxerCtx::new(Box::new(Mp4Demuxer::new()), Box::new(acc));

    debug!("read headers: {:?}", demuxer.read_headers().unwrap());
    debug!("global info: {:#?}", demuxer.info);

    loop {
        match demuxer.read_event() {
            Ok(event) => {
                // debug!("event: {:?}", event);
                match event {
                    Event::MoreDataNeeded(sz) => panic!("we needed more data: {} bytes", sz),
                    Event::NewStream(s) => panic!("new stream :{:?}", s),
                    Event::NewPacket(packet) => {
                        debug!("received packet: time={:?}", packet.t);
                    }
                    Event::Continue => {
                        continue;
                    }
                    Event::Eof => {
                        break;
                    }
                    _ => break,
                }
            }
            Err(e) => {
                error!("error: {:?}", e);
                break;
            }
        }
    }
}
