extern crate ffmpeg_next as ffmpeg;

use engine::audio::{self, AudioClip, AudioContext, AudioHandle};
use engine::texture_format::{PixelFormat, RawTextureData};
use engine_ffmpeg::AudioPlayer;
use ffmpeg::format::{input, Pixel};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg::ChannelLayout;
use std::env;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::rc::Rc;
use std::time::Duration;

use crate::resource_path;

// pub fn dump_frames(filename: &str) -> Result<(), ffmpeg::Error> {
//     ffmpeg::init().unwrap();

//     if let Ok(mut ictx) = input(&filename) {
//         let input = ictx
//             .streams()
//             .best(Type::Video)
//             .ok_or(ffmpeg::Error::StreamNotFound)?;
//         let video_stream_index = input.index();

//         let context_decoder =
//             ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
//         let mut decoder = context_decoder.decoder().video().unwrap();

//         let mut scaler = Context::get(
//             decoder.format(),
//             decoder.width(),
//             decoder.height(),
//             Pixel::RGB24,
//             decoder.width(),
//             decoder.height(),
//             Flags::BILINEAR,
//         )?;

//         let mut frame_index = 0;

//         let mut receive_and_process_decoded_frames =
//             |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
//                 let mut fail_count = 0;
//                 let mut decoded = Video::empty();
//                 // loop {
//                 //     match decoder.receive_frame(&mut decoded) {
//                 //         Ok(_) => {
//                 //             println!("---receiving frame...");
//                 //             let mut rgb_frame = Video::empty();
//                 //             scaler.run(&decoded, &mut rgb_frame).unwrap();
//                 //             save_file(&rgb_frame, frame_index).unwrap();
//                 //             frame_index += 1;
//                 //         }
//                 //         Err(e) => {
//                 //             // Handle other errors as needed
//                 //             println!("received error: {:?}", e);
//                 //             break;
//                 //         }
//                 //     }
//                 // }
//                 while decoder.receive_frame(&mut decoded).is_ok() {
//                     println!("---receiving frame...");
//                     let mut rgb_frame = Video::empty();
//                     scaler.run(&decoded, &mut rgb_frame).unwrap();
//                     save_file(&rgb_frame, frame_index).unwrap();
//                     frame_index += 1;
//                 }
//                 Ok(())
//             };

//         for (stream, packet) in ictx.packets() {
//             println!(
//                 "-- receiving packet: {} | {:?}",
//                 stream.index(),
//                 packet.pts()
//             );
//             if stream.index() == video_stream_index {
//                 println!("--- got video packet...");
//                 match decoder.send_packet(&packet) {
//                     Ok(()) => receive_and_process_decoded_frames(&mut decoder).unwrap(),
//                     Err(err) => println!("received err in send_packet: {:?}", err),
//                 }
//             }
//         }
//         decoder.send_eof()?;
//         receive_and_process_decoded_frames(&mut decoder)?;
//     }

//     Ok(())
// }

pub fn play_audio(
    filename: &str,
    context: &mut AudioContext<(), String>,
) -> Result<(), std::io::Error> {
    let clip = AudioPlayer::from_filename(filename).unwrap();

    //let clip = AudioClip::from_bytes(extracted_wav_bytes);
    let handle = AudioHandle::new();
    audio::test_audio(context, handle, None, Rc::new(clip));

    Ok(())
}
