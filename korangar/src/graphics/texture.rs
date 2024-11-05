use std::fmt::{Debug, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use derive_new::new;
use hashbrown::HashMap;
use wgpu::util::{DeviceExt, TextureDataOrder};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Device, Extent3d, Queue, ShaderStages, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
};

use crate::interface::layout::ScreenSize;

static TEXTURE_ID: AtomicU64 = AtomicU64::new(0);

pub struct Texture {
    id: u64,
    label: Option<String>,
    texture: wgpu::Texture,
    texture_view: TextureView,
    bind_group: BindGroup,
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
        let id = TEXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(descriptor);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: descriptor.label,
            layout: Self::bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            }],
        });

        Self {
            id,
            label,
            texture,
            texture_view,
            bind_group,
        }
    }

    pub fn new_with_data(device: &Device, queue: &Queue, descriptor: &TextureDescriptor, data: &[u8]) -> Self {
        let id = TEXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture_with_data(queue, descriptor, TextureDataOrder::LayerMajor, data);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: descriptor.label,
            layout: Self::bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            }],
        });

        Self {
            id,
            label,
            texture,
            texture_view,
            bind_group,
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_size(&self) -> Extent3d {
        self.texture.size()
    }

    pub fn get_format(&self) -> TextureFormat {
        self.texture.format()
    }

    pub fn get_texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn get_texture_view(&self) -> &TextureView {
        &self.texture_view
    }

    pub fn get_bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            })
        })
    }
}

pub struct CubeArrayTexture {
    label: Option<String>,
    texture: wgpu::Texture,
    texture_view: TextureView,
    texture_face_views: Vec<[TextureView; 6]>,
}

impl Debug for CubeArrayTexture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            None => write!(f, "CubeArrayTexture(\"Unknown\")"),
            Some(label) => write!(f, "CubeArrayTexture(\"{label}\")"),
        }
    }
}

impl CubeArrayTexture {
    pub(crate) fn new(
        device: &Device,
        texture_name: &str,
        face_dimensions: ScreenSize,
        format: TextureFormat,
        attachment_image_type: AttachmentTextureType,
        cube_count: u32,
    ) -> Self {
        let face_size = face_dimensions.width.max(face_dimensions.height);

        let descriptor = TextureDescriptor {
            label: Some(texture_name),
            size: Extent3d {
                width: face_size as u32,
                height: face_size as u32,
                depth_or_array_layers: 6 * cube_count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: attachment_image_type.into(),
            view_formats: &[],
        };

        let texture = device.create_texture(&descriptor);

        fn create_face_view(texture: &wgpu::Texture, cube_index: u32, face_index: u32) -> TextureView {
            texture.create_view(&TextureViewDescriptor {
                label: Some("cube array face view"),
                format: None,
                dimension: Some(TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: cube_index * 6 + face_index,
                array_layer_count: Some(1),
            })
        }

        let mut texture_face_views = Vec::with_capacity(cube_count as usize);
        for cube_idx in 0..cube_count {
            let face_views = [
                create_face_view(&texture, cube_idx, 0),
                create_face_view(&texture, cube_idx, 1),
                create_face_view(&texture, cube_idx, 2),
                create_face_view(&texture, cube_idx, 3),
                create_face_view(&texture, cube_idx, 4),
                create_face_view(&texture, cube_idx, 5),
            ];
            texture_face_views.push(face_views);
        }

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: Some("cube array view"),
            format: None,
            dimension: Some(TextureViewDimension::CubeArray),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: Some(6 * cube_count),
        });

        Self {
            label: descriptor.label.map(|l| l.to_string()),
            texture,
            texture_view,
            texture_face_views,
        }
    }

    pub fn get_texture_format(&self) -> TextureFormat {
        self.texture.format()
    }

    pub fn get_texture_view(&self) -> &TextureView {
        &self.texture_view
    }

    pub fn get_texture_face_view(&self, cube_index: usize, face_index: usize) -> &TextureView {
        &self.texture_face_views[cube_index][face_index]
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub(crate) enum AttachmentTextureType {
    PickerAttachment,
    ColorAttachment,
    ColorStorageAttachment,
    DepthAttachment,
    Depth,
}

impl From<AttachmentTextureType> for TextureUsages {
    fn from(value: AttachmentTextureType) -> Self {
        match value {
            AttachmentTextureType::PickerAttachment => {
                TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC
            }
            AttachmentTextureType::ColorAttachment => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            AttachmentTextureType::ColorStorageAttachment => {
                TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::STORAGE_BINDING
            }
            AttachmentTextureType::DepthAttachment => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            AttachmentTextureType::Depth => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        }
    }
}

pub struct AttachmentTexture {
    label: Option<String>,
    texture: wgpu::Texture,
    texture_view: TextureView,
    unpadded_size: Extent3d,
    bind_group: BindGroup,
}

impl Debug for AttachmentTexture {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.label {
            None => write!(formatter, "TextureAttachment(\"Unknown\")"),
            Some(label) => write!(formatter, "TextureAttachment(\"{label}\")"),
        }
    }
}

impl AttachmentTexture {
    pub fn new(device: &Device, mut descriptor: TextureDescriptor, padded_width: Option<u32>) -> Self {
        let unpadded_size = descriptor.size;

        if let Some(padded_width) = padded_width {
            descriptor.size.width = padded_width;
        }

        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            ..Default::default()
        });

        let sample_type = descriptor.format.sample_type(Some(TextureAspect::All), None).unwrap();

        let layout = if descriptor.sample_count == 1 {
            Self::bind_group_layout(device, sample_type, false)
        } else {
            Self::bind_group_layout(device, sample_type, true)
        };

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: descriptor.label,
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            }],
        });

        Self {
            label,
            texture,
            texture_view,
            unpadded_size,
            bind_group,
        }
    }

    pub fn get_unpadded_size(&self) -> Extent3d {
        self.unpadded_size
    }

    pub fn get_bytes_per_row(&self) -> Option<u32> {
        Some(self.texture.format().block_copy_size(None).unwrap() * self.texture.size().width)
    }

    pub fn get_format(&self) -> TextureFormat {
        self.texture.format()
    }

    pub fn get_texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn get_texture_view(&self) -> &TextureView {
        &self.texture_view
    }

    pub fn get_bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(device: &Device, mut sample_type: TextureSampleType, multisampled: bool) -> Arc<BindGroupLayout> {
        if multisampled
            && let TextureSampleType::Float { filterable } = &mut sample_type
            && *filterable
        {
            *filterable = false;
        }

        static LAYOUTS: OnceLock<Mutex<HashMap<(bool, TextureSampleType), Arc<BindGroupLayout>>>> = OnceLock::new();
        let layouts = LAYOUTS.get_or_init(|| Mutex::new(HashMap::new()));
        let lock = layouts.lock().unwrap();
        match lock.get(&(multisampled, sample_type)) {
            None => {
                let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type,
                            view_dimension: TextureViewDimension::D2,
                            multisampled,
                        },
                        count: None,
                    }],
                });
                let layout = Arc::new(layout);
                drop(lock);
                layouts.lock().unwrap().insert((multisampled, sample_type), layout.clone());
                layout
            }
            Some(layout) => layout.clone(),
        }
    }
}

#[derive(new)]
pub(crate) struct AttachmentTextureFactory<'a> {
    device: &'a Device,
    dimensions: ScreenSize,
    sample_count: u32,
    padded_width: Option<u32>,
}

impl<'a> AttachmentTextureFactory<'a> {
    pub(crate) fn new_attachment(
        &self,
        texture_name: &str,
        format: TextureFormat,
        attachment_image_type: AttachmentTextureType,
    ) -> AttachmentTexture {
        AttachmentTexture::new(
            self.device,
            TextureDescriptor {
                label: Some(texture_name),
                size: Extent3d {
                    width: self.dimensions.width as u32,
                    height: self.dimensions.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: self.sample_count,
                dimension: TextureDimension::D2,
                format,
                usage: attachment_image_type.into(),
                view_formats: &[],
            },
            self.padded_width,
        )
    }
}

pub struct StorageTexture {
    label: String,
    _texture: wgpu::Texture,
    texture_view: TextureView,
}

impl Debug for StorageTexture {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "StorageTexture(\"{}\")", self.label)
    }
}

impl StorageTexture {
    pub fn new(device: &Device, label: &str, width: u32, height: u32, format: TextureFormat) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: Some(label),
            ..Default::default()
        });

        Self {
            label: label.to_string(),
            _texture: texture,
            texture_view,
        }
    }

    pub fn get_texture_view(&self) -> &TextureView {
        &self.texture_view
    }
}
