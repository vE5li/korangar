use std::sync::Arc;

use derive_new::new;
use openh264::decoder::{DecodeOptions, Flush};
use wgpu::{Extent3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo};

use crate::graphics::Texture;

const DECODER_OPTIONS: DecodeOptions = DecodeOptions::new().flush_after_decode(Flush::NoFlush);
pub const FRAME_TIME_30_FPS: f64 = 1000.0 / 30.0;

#[derive(new)]
pub struct Video {
    width: u32,
    height: u32,
    h264_video: Option<H264Video>,
    texture: Arc<Texture>,
}

/// Currently always assumes 30 FPS.
#[derive(new)]
pub struct H264Video {
    bitstream: Vec<u8>,
    decoder: openh264::decoder::Decoder,
    next_frame_data: Vec<u8>,
    current_timestamp: f64,
    next_frame_timestamp: f64,
    offset: usize,
}

impl Video {
    /// Delta time is expected to be in seconds.
    pub fn should_update_frame(&mut self, delta_time: f64) -> bool {
        match self.h264_video.as_mut() {
            Some(h264_video) => {
                let ms = delta_time * 1000.0;
                h264_video.current_timestamp += ms;
                h264_video.current_timestamp >= h264_video.next_frame_timestamp
            }
            None => false,
        }
    }

    pub fn update_texture(&mut self, queue: &Queue) {
        let Some(h264_video) = &mut self.h264_video else {
            return;
        };

        queue.write_texture(
            TexelCopyTextureInfo {
                texture: self.texture.get_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &h264_video.next_frame_data,
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

    pub fn advance_frame(&mut self) {
        let Some(h264_video) = &mut self.h264_video else {
            return;
        };

        let decoded = loop {
            let Some(packet) = Self::next_packet(&h264_video.bitstream, &mut h264_video.offset) else {
                h264_video.current_timestamp = 0.0;
                h264_video.next_frame_timestamp = FRAME_TIME_30_FPS;
                return;
            };

            match h264_video.decoder.decode_with_options(packet, DECODER_OPTIONS) {
                Ok(Some(frame)) => break frame,
                Ok(None) => { /* frame not ready yet */ }
                Err(_) => return,
            }
        };

        decoded.write_rgba8(&mut h264_video.next_frame_data);

        h264_video.next_frame_timestamp += FRAME_TIME_30_FPS
    }

    pub fn get_texture(&self) -> &Arc<Texture> {
        &self.texture
    }

    pub fn next_packet<'a>(data: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
        let data = &data[*offset..];

        if data.is_empty() {
            return None;
        }

        match openh264::nal_units(data).next() {
            None => match *offset == 0 {
                true => None,
                false => {
                    // Reset video stream
                    *offset = 0;
                    None
                }
            },
            Some(packet) => {
                let size = packet.len();
                *offset += size;
                Some(packet)
            }
        }
    }
}
