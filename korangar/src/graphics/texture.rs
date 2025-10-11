use std::fmt::{Debug, Formatter};
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use hashbrown::HashMap;
use korangar_container::Cacheable;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Device, Extent3d, Origin3d, Queue, ShaderStages, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::graphics::{BindlessSupport, ScreenSize};

static TEXTURE_ID: AtomicU64 = AtomicU64::new(0);

pub struct Texture {
    id: u64,
    label: Option<String>,
    byte_size: usize,
    transparent: bool,
    texture: wgpu::Texture,
    texture_view: TextureView,
    bind_group: BindGroup,
}

impl Cacheable for Texture {
    fn size(&self) -> usize {
        self.byte_size
    }
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
    pub fn new(device: &Device, descriptor: &TextureDescriptor, transparent: bool) -> Self {
        let id = TEXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(descriptor);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            format: None,
            dimension: None,
            usage: None,
            aspect: TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: descriptor.label,
            layout: Self::bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            }],
        });

        let size = texture.size();
        let byte_size = size.width as usize * size.height as usize * texture.format().block_copy_size(None).unwrap() as usize;

        Self {
            id,
            byte_size,
            transparent,
            label,
            texture,
            texture_view,
            bind_group,
        }
    }

    pub fn new_with_data(device: &Device, queue: &Queue, descriptor: &TextureDescriptor, image_data: &[u8], transparent: bool) -> Self {
        let id = TEXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(descriptor);
        let format = texture.format();

        let (block_width, block_height) = format.block_dimensions();
        let block_size = format.block_copy_size(None).unwrap();

        let mut offset = 0;
        let mut mip_width = descriptor.size.width;
        let mut mip_height = descriptor.size.height;

        for mip_level in 0..descriptor.mip_level_count {
            let width_blocks = mip_width.div_ceil(block_width);
            let height_blocks = mip_height.div_ceil(block_height);

            let bytes_per_row = width_blocks * block_size;
            let mip_size = bytes_per_row * height_blocks;

            if offset + mip_size as usize <= image_data.len() {
                queue.write_texture(
                    TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level,
                        origin: Origin3d::ZERO,
                        aspect: TextureAspect::All,
                    },
                    &image_data[offset..offset + mip_size as usize],
                    TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: None,
                    },
                    Extent3d {
                        width: mip_width,
                        height: mip_height,
                        depth_or_array_layers: 1,
                    },
                );

                offset += mip_size as usize;
                mip_width = (mip_width / 2).max(1);
                mip_height = (mip_height / 2).max(1);
            } else {
                break;
            }
        }

        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            format: None,
            dimension: None,
            usage: None,
            aspect: TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: descriptor.label,
            layout: Self::bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&texture_view),
            }],
        });

        let size = texture.size();
        let byte_size = size.width as usize * size.height as usize * block_size as usize;

        Self {
            id,
            label,
            transparent,
            byte_size,
            texture,
            texture_view,
            bind_group,
        }
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_byte_size(&self) -> usize {
        self.byte_size
    }

    /// If  `true` a texture contains pixels that are neither fully opaque nor
    /// fully transparent.
    pub fn is_transparent(&self) -> bool {
        self.transparent
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

#[derive(Debug)]
pub struct TextureSet {
    textures: Vec<Arc<Texture>>,
    bind_group: Option<BindGroup>,
}

impl TextureSet {
    pub fn new(device: &Device, bindless_support: BindlessSupport, texture_set_size: u32, name: &str, textures: Vec<Arc<Texture>>) -> Self {
        let bind_group = match bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                let mut views = Vec::from_iter(textures.iter().map(|texture| texture.get_texture_view()));

                if bindless_support == BindlessSupport::Limited && !textures.is_empty() {
                    let default_view = textures[0].get_texture_view();
                    views.resize_with(texture_set_size as usize, || default_view);
                }

                let bind_group = device.create_bind_group(&BindGroupDescriptor {
                    label: Some(name),
                    layout: Self::bind_group_layout(device, texture_set_size),
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureViewArray(&views),
                    }],
                });

                Some(bind_group)
            }
            BindlessSupport::None => None,
        };

        Self { textures, bind_group }
    }

    pub fn get_texture_bind_group(&self, texture_id: i32) -> &BindGroup {
        self.textures[texture_id as usize].get_bind_group()
    }

    pub fn get_bind_group(&self) -> Option<&BindGroup> {
        self.bind_group.as_ref()
    }

    pub fn bind_group_layout(device: &Device, texture_set_size: u32) -> &'static BindGroupLayout {
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
                    count: NonZeroU32::new(texture_set_size),
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
                usage: None,
                aspect: TextureAspect::All,
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
            usage: None,
            aspect: TextureAspect::All,
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
            AttachmentTextureType::DepthAttachment => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            AttachmentTextureType::Depth => TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        }
    }
}

pub struct AttachmentTexture {
    label: Option<String>,
    texture: wgpu::Texture,
    texture_view: TextureView,
    array_texture_views: Vec<TextureView>,
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

        let (view_dimension, array_layer_count) = if descriptor.size.depth_or_array_layers == 1 {
            (TextureViewDimension::D2, None)
        } else {
            (TextureViewDimension::D2Array, Some(descriptor.size.depth_or_array_layers))
        };

        let label = descriptor.label.map(|label| label.to_string());
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_view(&TextureViewDescriptor {
            label: descriptor.label,
            format: None,
            dimension: Some(view_dimension),
            usage: None,
            aspect: TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count,
        });

        let mut array_texture_views = Vec::default();

        if descriptor.size.depth_or_array_layers > 1 {
            for layer in 0..descriptor.size.depth_or_array_layers {
                array_texture_views.push(texture.create_view(&TextureViewDescriptor {
                    label: descriptor.label,
                    format: None,
                    dimension: Some(TextureViewDimension::D2),
                    usage: None,
                    aspect: TextureAspect::default(),
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                }));
            }
        }

        let sample_type = descriptor.format.sample_type(Some(TextureAspect::All), None).unwrap();

        let layout = if descriptor.sample_count == 1 {
            Self::bind_group_layout(device, view_dimension, sample_type, false)
        } else {
            Self::bind_group_layout(device, view_dimension, sample_type, true)
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
            array_texture_views,
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

    pub fn get_array_texture_view(&self, index: usize) -> &TextureView {
        &self.array_texture_views[index]
    }

    pub fn get_bind_group(&self) -> &BindGroup {
        &self.bind_group
    }

    pub fn bind_group_layout(
        device: &Device,
        view_dimension: TextureViewDimension,
        mut sample_type: TextureSampleType,
        multisampled: bool,
    ) -> Arc<BindGroupLayout> {
        if multisampled
            && let TextureSampleType::Float { filterable } = &mut sample_type
            && *filterable
        {
            *filterable = false;
        }

        #[allow(clippy::type_complexity)]
        static LAYOUTS: OnceLock<Mutex<HashMap<(TextureViewDimension, TextureSampleType, bool), Arc<BindGroupLayout>>>> = OnceLock::new();
        let layouts = LAYOUTS.get_or_init(|| Mutex::new(HashMap::new()));
        let lock = layouts.lock().unwrap();
        match lock.get(&(view_dimension, sample_type, multisampled)) {
            None => {
                let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type,
                            view_dimension,
                            multisampled,
                        },
                        count: None,
                    }],
                });
                let layout = Arc::new(layout);
                drop(lock);
                layouts
                    .lock()
                    .unwrap()
                    .insert((view_dimension, sample_type, multisampled), layout.clone());
                layout
            }
            Some(layout) => layout.clone(),
        }
    }
}

pub(crate) struct AttachmentTextureFactory<'a> {
    device: &'a Device,
    dimensions: ScreenSize,
    sample_count: u32,
    padded_width: Option<u32>,
}

impl<'a> AttachmentTextureFactory<'a> {
    pub(crate) fn new(device: &'a Device, dimensions: ScreenSize, sample_count: u32, padded_width: Option<u32>) -> Self {
        Self {
            device,
            dimensions,
            sample_count,
            padded_width,
        }
    }
}

impl AttachmentTextureFactory<'_> {
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

    pub(crate) fn new_attachment_array(
        &self,
        texture_name: &str,
        format: TextureFormat,
        attachment_image_type: AttachmentTextureType,
        count: u32,
    ) -> AttachmentTexture {
        AttachmentTexture::new(
            self.device,
            TextureDescriptor {
                label: Some(texture_name),
                size: Extent3d {
                    width: self.dimensions.width as u32,
                    height: self.dimensions.height as u32,
                    depth_or_array_layers: count,
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
            format: None,
            dimension: None,
            usage: None,
            aspect: TextureAspect::default(),
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
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
