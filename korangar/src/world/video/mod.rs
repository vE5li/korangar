use std::sync::Arc;

use korangar_video::{Decoder, Error, Picture};
use wgpu::{Extent3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo};

use crate::graphics::Texture;

pub struct Video {
    width: u32,
    height: u32,
    timescale: f64,
    frames: Vec<VideoFrame>,
    decoder: Decoder,
    current_timestamp: f64,
    last_timestamp: i64,
    next_picture_timestamp: i64,
    next_frame_index: usize,
    next_picture: Option<Picture>,
    rgba_data: Vec<u8>,
    texture: Arc<Texture>,
}

pub struct VideoFrame {
    pub timestamp: i64,
    pub packet: Arc<[u8]>,
}

impl Video {
    pub fn new(width: u32, height: u32, timescale: f64, frames: Vec<VideoFrame>, texture: Arc<Texture>) -> Self {
        Self {
            width,
            height,
            timescale,
            frames,
            decoder: Decoder::new().expect("Can't create decoder"),
            current_timestamp: 0.0,
            last_timestamp: -1,
            next_picture_timestamp: -1,
            next_frame_index: 0,
            next_picture: None,
            rgba_data: vec![0; width as usize * height as usize * 4],
            texture,
        }
    }

    pub fn check_for_next_frame(&mut self) {
        if self.frames.is_empty() || self.next_picture_timestamp >= 0 {
            return;
        }

        match self.decoder.get_picture(self.next_picture.take()) {
            Ok(picture) => {
                let next_timestamp = picture.timestamp().unwrap_or(0);

                if next_timestamp < self.last_timestamp {
                    // Video is looping.
                    self.current_timestamp = 0.0;
                }
                self.last_timestamp = next_timestamp;

                self.next_picture_timestamp = next_timestamp;
                self.next_picture = Some(picture);
            }
            Err(Error::Again) => loop {
                match self.decoder.send_pending_data() {
                    Ok(_) => { /* No pending data left */ }
                    Err(Error::Again) => break,
                    Err(_error) => {
                        /* Decoding error. Nothing we can do. */
                        return;
                    }
                }

                let Some(frame) = self.frames.get(self.next_frame_index) else {
                    self.next_frame_index = 0;
                    continue;
                };
                self.next_frame_index += 1;

                let timestamp = (frame.timestamp as f64 / self.timescale).floor() as i64;

                match self.decoder.send_data(Arc::clone(&frame.packet), None, Some(timestamp), None) {
                    Ok(_) => continue,
                    Err(Error::Again) => match self.decoder.send_pending_data() {
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
        if self.frames.is_empty() {
            return false;
        }

        let ms = delta_time * 1000.0;
        self.current_timestamp += ms;

        self.next_picture_timestamp >= 0 && self.current_timestamp.floor() as i64 >= self.next_picture_timestamp
    }

    pub fn update_texture(&mut self, queue: &Queue) {
        self.next_picture_timestamp = -1;

        let Some(picture) = self.next_picture.as_ref() else {
            return;
        };

        picture.write_rgba8(&mut self.rgba_data);

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: self.texture.get_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.rgba_data.as_slice(),
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
