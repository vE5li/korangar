use std::sync::Arc;

use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::FileLoader;
use openh264::formats::YUVSource;
use wgpu::TextureFormat;

use crate::loaders::{FALLBACK_PNG_FILE, GameFileLoader, TextureLoader, video_file_h264_name};
use crate::world::{FRAME_TIME_30_FPS, H264Video, Video};

const BIK_FILE_ENDING: &str = ".bik";

#[derive(new)]
pub struct VideoLoader {
    queue: Arc<wgpu::Queue>,
    game_file_loader: Arc<GameFileLoader>,
    texture_loader: Arc<TextureLoader>,
}

impl VideoLoader {
    fn convert_frame(&self, decoded: &openh264::decoder::DecodedYUV, old_buffer: Option<Vec<u8>>) -> Vec<u8> {
        let (width, height) = decoded.dimensions();
        let mut rgba = old_buffer.unwrap_or_else(|| vec![0u8; width * height * 4]);
        decoded.write_rgba8(&mut rgba);
        rgba
    }

    pub fn is_video_file(&self, path: &str) -> bool {
        path.ends_with(BIK_FILE_ENDING)
    }

    fn load_video(&self, path: &str) -> Option<Video> {
        let video_file_name = video_file_h264_name(path);
        let path = format!("data\\video\\{video_file_name}");

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load video data from {}", path.magenta()));

        let Ok(mut h264_data) = self.game_file_loader.get(&path) else {
            #[cfg(feature = "debug")]
            print_debug!("Could not find H264 video file: {}", path);
            return None;
        };

        // The h264 file contains a blake3 hash value at the end, which we cut off.
        h264_data.truncate(h264_data.len() - blake3::OUT_LEN);

        let mut decoder = openh264::decoder::Decoder::new().expect("Can't create h264 decoder");
        let mut offset = 0;

        let decoded = loop {
            let Some(packet) = Video::next_packet(&h264_data, &mut offset) else {
                #[cfg(feature = "debug")]
                print_debug!("No NAL unit found");
                return None;
            };

            match decoder.decode(packet) {
                Ok(Some(frame)) => break frame,
                Ok(None) => { /* frame not ready yet */ }
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    print_debug!("Can't decode frame: {}", _error);
                    return None;
                }
            }
        };

        let (width, height) = decoded.dimensions();
        let (width, height) = (width as u32, height as u32);

        let mut frame_data = self.convert_frame(&decoded, None);

        let texture = self
            .texture_loader
            .create_raw(&video_file_name, width, height, 1, TextureFormat::Rgba8UnormSrgb, false);

        let mut next_frame_timestamp = None;

        loop {
            let Some(packet) = Video::next_packet(&h264_data, &mut offset) else {
                break;
            };

            match decoder.decode(packet) {
                Ok(Some(decoded_frame)) => {
                    next_frame_timestamp = Some(decoded_frame.timestamp().as_millis());
                    frame_data = self.convert_frame(&decoded_frame, Some(frame_data));
                    break;
                }
                Ok(None) => { /* frame not ready yet */ }
                Err(_error) => {
                    #[cfg(feature = "debug")]
                    print_debug!("Can't decode frame: {}", _error);
                    return None;
                }
            }
        }

        let mut video = match next_frame_timestamp {
            // Video has only one frame
            None => Video::new(width, height, None, texture),
            // Video has multiple frames
            Some(_) => Video::new(
                width,
                height,
                Some(H264Video::new(h264_data, decoder, frame_data, 0.0, FRAME_TIME_30_FPS, offset)),
                texture,
            ),
        };

        video.update_texture(&self.queue);
        video.advance_frame();

        #[cfg(feature = "debug")]
        timer.stop();

        Some(video)
    }

    pub fn load(&self, path: &str) -> Video {
        match self.load_video(path) {
            Some(video) => video,
            None => {
                #[cfg(feature = "debug")]
                print_debug!("Failed to load video. Using placeholder texture");

                let (image, _) = self
                    .texture_loader
                    .load_texture_data(FALLBACK_PNG_FILE, false)
                    .expect("can't load fallback PNG file");
                let (width, height) = image.dimensions();
                let texture = self.texture_loader.create_color(path, image, false);

                Video::new(width, height, None, texture)
            }
        }
    }
}
