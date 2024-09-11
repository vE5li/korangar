use std::fmt::{Debug, Formatter};
use std::num::NonZeroU32;
use std::sync::{Arc, OnceLock};

use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Device, Features, Queue, ShaderStages, TextureDescriptor, TextureSampleType, TextureViewDescriptor, TextureViewDimension,
};

use crate::graphics::features_supported;
use crate::MAX_BINDING_TEXTURE_ARRAY_COUNT;

pub struct Texture {
    label: Option<String>,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
}

impl Debug for Texture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            None => write!(f, "Texture(\"Unknown\")"),
            Some(label) => write!(f, "Texture(\"{label}\")"),
        }
    }
}

impl Texture {
    pub fn new(device: &Device, descriptor: &TextureDescriptor) -> Self {
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(descriptor);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            ..Default::default()
        });

        Self {
            label,
            texture,
            texture_view,
        }
    }

    pub fn new_with_data(device: &Device, queue: &Queue, descriptor: &TextureDescriptor, data: &[u8]) -> Self {
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture_with_data(queue, descriptor, TextureDataOrder::LayerMajor, data);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            ..Default::default()
        });

        Self {
            label,
            texture,
            texture_view,
        }
    }

    pub fn get_extend(&self) -> wgpu::Extent3d {
        self.texture.size()
    }

    pub fn get_texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn get_texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }
}

pub struct TextureGroup {
    _textures: Vec<Arc<Texture>>,
    bind_group: BindGroup,
}

impl TextureGroup {
    pub fn bind_group_layout(device: &Device) -> &BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("texture group"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: NonZeroU32::new(MAX_BINDING_TEXTURE_ARRAY_COUNT as u32),
                }],
            })
        })
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn new(device: &Device, label: &str, textures: Vec<Arc<Texture>>) -> Self {
        let texture_count = textures.len();
        let mut texture_views: Vec<&wgpu::TextureView> = textures
            .iter()
            .take(MAX_BINDING_TEXTURE_ARRAY_COUNT.min(texture_count))
            .map(|texture| texture.get_texture_view())
            .collect();

        if !features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
            for _ in 0..MAX_BINDING_TEXTURE_ARRAY_COUNT.saturating_sub(texture_count) {
                texture_views.push(texture_views[0]);
            }
        }

        let layout = Self::bind_group_layout(device);
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureViewArray(&texture_views),
            }],
        });

        Self {
            _textures: textures,
            bind_group,
        }
    }
}
