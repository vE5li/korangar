use std::iter;
use std::sync::Arc;

#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_loaders::FileLoader;
use korangar_video::ivf::Ivf;
use wgpu::TextureFormat;

use crate::loaders::{FALLBACK_PNG_FILE, GameFileLoader, TextureLoader, video_file_ivf_name};
use crate::world::{Video, VideoFrame};

const BIK_FILE_ENDING: &str = ".bik";

pub struct VideoLoader {
    game_file_loader: Arc<GameFileLoader>,
    texture_loader: Arc<TextureLoader>,
}

impl VideoLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>, texture_loader: Arc<TextureLoader>) -> Self {
        Self {
            game_file_loader,
            texture_loader,
        }
    }
}

impl VideoLoader {
    pub fn is_video_file(&self, path: &str) -> bool {
        path.ends_with(BIK_FILE_ENDING)
    }

    fn load_video(&self, path: &str) -> Option<Video> {
        let video_file_name = video_file_ivf_name(path);
        let path = format!("data\\video\\{video_file_name}");

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load video data from {}", path.magenta()));

        let Ok(mut file_data) = self.game_file_loader.get(&path) else {
            #[cfg(feature = "debug")]
            print_debug!("Could not find IVF video file `{}`", path);
            return None;
        };

        // The IVF file contains a blake3 hash value at the end, which we cut off.
        file_data.truncate(file_data.len() - blake3::OUT_LEN);

        let mut ivf = match Ivf::new(file_data.as_slice()) {
            Ok(ivf_file) => ivf_file,
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!("Can't open IVF video file `{}`: {}", path, _error);
                return None;
            }
        };

        if ivf.four_cc() != [b'A', b'V', b'0', b'1'] && ivf.four_cc() != [b'a', b'v', b'0', b'1'] {
            #[cfg(feature = "debug")]
            print_debug!("IVF is not an AV1 video file: {:?}", ivf.four_cc());
            return None;
        }

        let width = ivf.width() as u32;
        let height = ivf.height() as u32;
        let timescale = ivf.header().timebase_numerator as f64 / ivf.header().timebase_denominator as f64;

        let texture = self
            .texture_loader
            .create_raw(&video_file_name, width, height, 1, TextureFormat::Rgba8UnormSrgb, false);

        let frames = Vec::from_iter(iter::from_fn(|| {
            ivf.read_frame().ok().flatten().map(|frame| VideoFrame {
                timestamp: frame.timestamp as i64,
                packet: frame.packet.into(),
            })
        }));

        let video = Video::new(width, height, timescale, frames, texture);

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

                Video::new(width, height, 1.0, Vec::new(), texture)
            }
        }
    }
}
