use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferUsages, Device, Queue, ShaderStages};

use super::Buffer;

/// Per glyph data that the Slug pixel shader reads.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SlugGlyph {
    /// The scale (xy) and offset (zw) that map an em space coordinate to a
    /// band index.
    pub band_transform: [f32; 4],
    /// The index of the first band header of the glyph in the band buffer.
    pub band_base: u32,
    /// The index of the last vertical band.
    pub band_max_x: u32,
    /// The index of the last horizontal band.
    pub band_max_y: u32,
    pub padding: u32,
}

/// The GPU buffers that hold the glyph outlines of all loaded fonts for the
/// Slug font rendering algorithm.
pub struct SlugFont {
    curves: Buffer<[f32; 4]>,
    bands: Buffer<u32>,
    glyphs: Buffer<SlugGlyph>,
}

impl SlugFont {
    pub fn new(device: &Device, queue: &Queue, curves: &[[f32; 4]], bands: &[u32], glyphs: &[SlugGlyph]) -> Self {
        let usage = BufferUsages::COPY_DST | BufferUsages::STORAGE;

        Self {
            curves: Buffer::with_data(device, queue, "slug curves", usage, curves),
            bands: Buffer::with_data(device, queue, "slug bands", usage, bands),
            glyphs: Buffer::with_data(device, queue, "slug glyphs", usage, glyphs),
        }
    }

    /// A font without glyphs, used until the real font is bound.
    pub fn placeholder(device: &Device, queue: &Queue) -> Self {
        Self::new(device, queue, &[[0.0; 4]], &[0], &[SlugGlyph::default()])
    }

    /// The bind group layout entries for the curve, band and glyph buffer,
    /// starting at `first_binding`.
    pub fn bind_group_layout_entries(first_binding: u32) -> [BindGroupLayoutEntry; 3] {
        let entry = |binding| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        [entry(first_binding), entry(first_binding + 1), entry(first_binding + 2)]
    }

    /// The binding resources in the order of
    /// [`SlugFont::bind_group_layout_entries`].
    pub fn binding_resources(&self) -> [BindingResource<'_>; 3] {
        [
            self.curves.as_entire_binding(),
            self.bands.as_entire_binding(),
            self.glyphs.as_entire_binding(),
        ]
    }
}
