use std::io::Cursor;
use std::sync::Arc;

use derive_new::new;
use korangar_video::ivf::Ivf;
use korangar_video::{Decoder, Error, Picture};
use wgpu::{Extent3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo};

use crate::graphics::Texture;

#[derive(new)]
pub struct Video {
    width: u32,
    height: u32,
    av1_video: Option<AV1Video>,
    texture: Arc<Texture>,
}

#[derive(new)]
pub struct AV1Video {
    ivf: Ivf<Cursor<Vec<u8>>>,
    decoder: Decoder,
    current_timestamp: f64,
    last_timestamp: i64,
    next_picture_timestamp: i64,
    timescale: f64,
    next_picture: Option<Picture>,
    rgba_data: Vec<u8>,
}

impl Video {
    pub fn check_for_next_frame(&mut self) {
        let Some(av1_video) = &mut self.av1_video else {
            return;
        };

        if av1_video.next_picture_timestamp >= 0 {
            return;
        }

        match av1_video.decoder.get_picture(av1_video.next_picture.take()) {
            Ok(picture) => {
                let next_timestamp = picture.timestamp().unwrap_or(0);

                if next_timestamp < av1_video.last_timestamp {
                    // Video is looping.
                    av1_video.current_timestamp = 0.0;
                }
                av1_video.last_timestamp = next_timestamp;

                av1_video.next_picture_timestamp = next_timestamp;
                av1_video.next_picture = Some(picture);
            }
            Err(Error::Again) => loop {
                match av1_video.decoder.send_pending_data() {
                    Ok(_) => { /* No pending data left */ }
                    Err(Error::Again) => break,
                    Err(_error) => {
                        /* Decoding error. Nothing we can do. */
                        return;
                    }
                }

                let Ok(Some(frame)) = av1_video.ivf.read_frame() else {
                    let _ = av1_video.ivf.reset();
                    continue;
                };

                let timestamp = (frame.timestamp as f64 / av1_video.timescale).floor() as i64;

                match av1_video.decoder.send_data(frame.packet, None, Some(timestamp), None) {
                    Ok(_) => continue,
                    Err(Error::Again) => match av1_video.decoder.send_pending_data() {
                        Ok(_) | Err(Error::Again) => break,
                        Err(_error) => {
                            /* Decoding error. Nothing we can do. */
                            return;
                        }
                    },
                    Err(_error) => {
                        /* Decoding error. Nothing we can do. */
                        return;
                    }
                }
            },
            Err(_error) => { /* Decoding error. Nothing we can do. */ }
        };
    }

    /// Delta time is expected to be in seconds.
    pub fn should_show_next_frame(&mut self, delta_time: f64) -> bool {
        match self.av1_video.as_mut() {
            Some(av1_video) => {
                let ms = delta_time * 1000.0;
                av1_video.current_timestamp += ms;
                av1_video.next_picture_timestamp >= 0 && av1_video.current_timestamp.floor() as i64 >= av1_video.next_picture_timestamp
            }
            _ => false,
        }
    }

    pub fn update_texture(&mut self, queue: &Queue) {
        let Some(av1_video) = &mut self.av1_video else {
            return;
        };

        av1_video.next_picture_timestamp = -1;

        let Some(picture) = av1_video.next_picture.as_ref() else {
            return;
        };

        picture.write_rgba8(&mut av1_video.rgba_data);

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: self.texture.get_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            av1_video.rgba_data.as_slice(),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width * 4),
                rows_per_image: Some(self.height),
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn get_texture(&self) -> &Arc<Texture> {
        &self.texture
    }
}
