extern crate ffmpeg_next as ffmpeg;

use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct VideoStream {
    frames: Arc<Mutex<VecDeque<Video>>>,
    decoder_thread: Option<thread::JoinHandle<()>>,
    exit_flag: Arc<Mutex<bool>>,
}

impl Drop for VideoStream {
    fn drop(&mut self) {
        //TODO: How to better handle failure to close the stream?
        self.close_stream().unwrap();
    }
}
impl VideoStream {
    pub fn new(filename: &str, prefetch_count: usize) -> Result<Self, ffmpeg::Error> {
        ffmpeg::init().unwrap();

        let frames = Arc::new(Mutex::new(VecDeque::new()));
        let frames_clone = Arc::clone(&frames);

        let exit_flag = Arc::new(Mutex::new(false));
        let exit_flag_clone = Arc::clone(&exit_flag);
        let filename_clone = filename.to_string();
        let decoder_thread = thread::spawn(move || {
            decode_frames(
                &filename_clone,
                exit_flag_clone,
                frames_clone,
                prefetch_count,
            )
            .unwrap();
        });

        Ok(Self {
            decoder_thread: Some(decoder_thread),
            frames,
            exit_flag,
        })
    }

    pub fn get_next_frame(&mut self) -> Option<Video> {
        self.frames
            .lock()
            .expect("Unable to unwrap frame buffer")
            .pop_front()
    }

    pub fn close_stream(&mut self) -> Result<(), ffmpeg::Error> {
        *self.exit_flag.lock().expect("Unable to set exit flag!") = true;
        if let Some(handle) = self.decoder_thread.take() {
            handle.join().expect("Unable to join handle");
        }
        Ok(())
    }
}

pub fn decode_frames(
    filename: &str,
    exit_flag: Arc<Mutex<bool>>,
    frames: Arc<Mutex<VecDeque<Video>>>,
    prefetch_count: usize,
) -> Result<(), ffmpeg::Error> {
    let mut ictx = input(&filename)?;
    let input = ictx
        .streams()
        .best(Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_stream_index = input.index();

    let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGBA,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    for (_, packet) in ictx
        .packets()
        .filter(|(stream, _)| stream.index() == video_stream_index)
    {
        if frames.lock().expect("Unable to get frame count").len() > prefetch_count {
            thread::sleep(Duration::from_secs_f32(0.1 as f32));
        }
        if *exit_flag.lock().expect("Unable to read exit flag!") {
            break;
        }
        decoder.send_packet(&packet)?;
        let mut decoded = Video::empty();
        while decoder.receive_frame(&mut decoded).is_ok() {
            let mut rgb_frame = Video::empty();
            scaler.run(&decoded, &mut rgb_frame)?;
            frames
                .lock()
                .expect("Unable to push to frames")
                .push_back(rgb_frame);
        }
    }
    decoder.send_eof()?;
    Ok(())
}
