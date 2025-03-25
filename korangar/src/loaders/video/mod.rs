use std::io::Cursor;
use std::sync::Arc;

use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::FileLoader;
use korangar_video::ivf::Ivf;
use wgpu::TextureFormat;

use crate::loaders::{FALLBACK_PNG_FILE, GameFileLoader, TextureLoader, video_file_ivf_name};
use crate::world::{AV1Video, Video};

const BIK_FILE_ENDING: &str = ".bik";

#[derive(new)]
pub struct VideoLoader {
    game_file_loader: Arc<GameFileLoader>,
    texture_loader: Arc<TextureLoader>,
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

        let Ok(mut ivf_data) = self.game_file_loader.get(&path) else {
            #[cfg(feature = "debug")]
            print_debug!("Could not find IVF video file `{}`", path);
            return None;
        };

        // The IVF file contains a blake3 hash value at the end, which we cut off.
        ivf_data.truncate(ivf_data.len() - blake3::OUT_LEN);

        let ivf_file = match Ivf::new(Cursor::new(ivf_data)) {
            Ok(ivf_file) => ivf_file,
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!("Can't open IVF video file `{}`: {}", path, _error);
                return None;
            }
        };

        let Ok(decoder) = korangar_video::Decoder::new() else {
            #[cfg(feature = "debug")]
            print_debug!("Can't initialize decoder");
            return None;
        };

        if ivf_file.four_cc() != [b'A', b'V', b'0', b'1'] && ivf_file.four_cc() != [b'a', b'v', b'0', b'1'] {
            #[cfg(feature = "debug")]
            print_debug!("IVF is not an AV1 video file: {:?}", ivf_file.four_cc());
            return None;
        }

        let width = ivf_file.width() as u32;
        let height = ivf_file.height() as u32;
        let timescale = ivf_file.header().timebase_numerator as f64 / ivf_file.header().timebase_denominator as f64;

        let texture = self
            .texture_loader
            .create_raw(&video_file_name, width, height, 1, TextureFormat::Rgba8UnormSrgb, false);

        let video = Video::new(
            width,
            height,
            Some(AV1Video::new(ivf_file, decoder, 0.0, -1, -1, timescale, None, vec![
                0;
                width as usize * height as usize
                    * 4
            ])),
            texture,
        );

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
