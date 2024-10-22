mod buffer;
mod cameras;
mod color;
mod engine;
#[cfg(feature = "debug")]
mod error;
mod features;
mod graphic_settings;
mod instruction;
mod particles;
mod passes;
mod picker_target;
#[cfg(feature = "debug")]
mod render_settings;
mod sampler;
mod smoothed;
mod surface;
mod texture;
mod vertices;

use std::num::{NonZeroU32, NonZeroU64};
use std::sync::{Arc, OnceLock};

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, SquareMatrix, Zero};
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, CommandEncoder, Device,
    Extent3d, Queue, Sampler, SamplerBindingType, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureViewDimension,
};

pub use self::buffer::Buffer;
pub use self::cameras::*;
pub use self::color::*;
pub use self::engine::{GraphicsEngine, GraphicsEngineDescriptor};
#[cfg(feature = "debug")]
pub use self::error::error_handler;
pub use self::features::*;
pub use self::graphic_settings::*;
pub use self::instruction::*;
pub use self::particles::*;
pub use self::picker_target::PickerTarget;
#[cfg(feature = "debug")]
pub use self::render_settings::*;
pub use self::smoothed::*;
pub use self::surface::*;
pub use self::texture::*;
pub use self::vertices::*;
use crate::graphics::sampler::create_new_sampler;
use crate::interface::layout::ScreenSize;
use crate::loaders::TextureLoader;
use crate::NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS;

pub const LIGHT_ATTACHMENT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
};

pub const WATER_ATTACHMENT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::ReverseSubtract,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
};

pub const EFFECT_ATTACHMENT_BLEND: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Max,
    },
};

/// Trait to prepare all GPU data of contexts, computer and renderer.
pub(crate) trait Prepare {
    /// Prepares the GPU data.
    fn prepare(&mut self, _device: &Device, _instructions: &RenderInstruction);

    /// Stages the prepared data inside the staging belt.
    fn upload(&mut self, _device: &Device, _staging_belt: &mut StagingBelt, _command_encoder: &mut CommandEncoder);
}

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct GlobalUniforms {
    view_projection: [[f32; 4]; 4],
    inverse_view_projection: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    indicator_positions: [[f32; 4]; 4],
    indicator_color: [f32; 4],
    ambient_color: [f32; 4],
    pointer_position: [u32; 2],
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
    padding: [u32; 3],
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct DebugUniforms {
    show_diffuse_buffer: u32,
    show_normal_buffer: u32,
    show_water_buffer: u32,
    show_depth_buffer: u32,
    show_picker_buffer: u32,
    show_shadow_buffer: u32,
    show_font_atlas: u32,
    show_point_shadow: u32,
}

/// Holds all GPU resources that are shared by multiple passes.
pub(crate) struct GlobalContext {
    pub(crate) surface_texture_format: TextureFormat,
    pub(crate) solid_pixel_texture: Arc<Texture>,
    pub(crate) walk_indicator_texture: Arc<Texture>,
    pub(crate) depth_texture: AttachmentTexture,
    pub(crate) picker_buffer_texture: AttachmentTexture,
    pub(crate) diffuse_buffer_texture: AttachmentTexture,
    pub(crate) normal_buffer_texture: AttachmentTexture,
    pub(crate) water_buffer_texture: AttachmentTexture,
    pub(crate) depth_buffer_texture: AttachmentTexture,
    pub(crate) interface_buffer_texture: AttachmentTexture,
    pub(crate) directional_shadow_map_texture: AttachmentTexture,
    pub(crate) point_shadow_map_textures: [CubeTexture; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS],
    pub(crate) global_uniforms_buffer: Buffer<GlobalUniforms>,
    #[cfg(feature = "debug")]
    pub(crate) debug_uniforms_buffer: Buffer<DebugUniforms>,
    pub(crate) picker_value_buffer: Buffer<u64>,
    pub(crate) nearest_sampler: Sampler,
    pub(crate) linear_sampler: Sampler,
    pub(crate) texture_sampler: Sampler,
    pub(crate) global_bind_group: BindGroup,
    pub(crate) picker_bind_group: BindGroup,
    pub(crate) screen_bind_group: BindGroup,
    pub(crate) screen_size: ScreenSize,
    pub(crate) directional_shadow_size: ScreenSize,
    pub(crate) point_shadow_size: ScreenSize,
    global_uniforms: GlobalUniforms,
    #[cfg(feature = "debug")]
    debug_uniforms: DebugUniforms,
}

impl Prepare for GlobalContext {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        let (indicator_positions, indicator_color) = instructions
            .indicator
            .as_ref()
            .map_or((Matrix4::zero(), Color::WHITE), |indicator| {
                (
                    Matrix4::from_cols(
                        indicator.upper_left.to_homogeneous(),
                        indicator.upper_right.to_homogeneous(),
                        indicator.lower_left.to_homogeneous(),
                        indicator.lower_right.to_homogeneous(),
                    ),
                    indicator.color,
                )
            });

        let view_projection = instructions.uniforms.projection_matrix * instructions.uniforms.view_matrix;
        self.global_uniforms = GlobalUniforms {
            view_projection: view_projection.into(),
            inverse_view_projection: view_projection.invert().unwrap_or_else(Matrix4::identity).into(),
            view: instructions.uniforms.view_matrix.into(),
            indicator_positions: indicator_positions.into(),
            indicator_color: indicator_color.components_linear(),
            ambient_color: instructions.uniforms.ambient_light_color.components_linear(),
            pointer_position: [instructions.picker_position.left as u32, instructions.picker_position.top as u32],
            animation_timer: instructions.uniforms.animation_timer,
            day_timer: instructions.uniforms.day_timer,
            water_level: instructions.uniforms.water_level,
            padding: Default::default(),
        };

        #[cfg(feature = "debug")]
        {
            self.debug_uniforms = DebugUniforms {
                show_diffuse_buffer: instructions.render_settings.show_diffuse_buffer as u32,
                show_normal_buffer: instructions.render_settings.show_normal_buffer as u32,
                show_water_buffer: instructions.render_settings.show_water_buffer as u32,
                show_depth_buffer: instructions.render_settings.show_depth_buffer as u32,
                show_picker_buffer: instructions.render_settings.show_picker_buffer as u32,
                show_shadow_buffer: instructions.render_settings.show_shadow_buffer as u32,
                show_font_atlas: instructions.render_settings.show_font_atlas as u32,
                show_point_shadow: instructions.render_settings.show_point_shadow.map(|value| value.get()).unwrap_or(0),
            };
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let recreated = self
            .global_uniforms_buffer
            .write(device, staging_belt, command_encoder, &[self.global_uniforms]);

        #[cfg(feature = "debug")]
        let recreated = recreated
            | self
                .debug_uniforms_buffer
                .write(device, staging_belt, command_encoder, &[self.debug_uniforms]);

        if recreated {
            self.global_bind_group = Self::create_global_bind_group(
                device,
                &self.global_uniforms_buffer,
                #[cfg(feature = "debug")]
                &self.debug_uniforms_buffer,
                &self.nearest_sampler,
                &self.linear_sampler,
                &self.texture_sampler,
            );
        }
    }
}

impl GlobalContext {
    fn new(
        device: &Device,
        queue: &Queue,
        texture_loader: &TextureLoader,
        surface_texture_format: TextureFormat,
        screen_size: ScreenSize,
        shadow_detail: ShadowDetail,
        texture_sampler: TextureSamplerType,
    ) -> Self {
        let directional_shadow_size = ScreenSize::uniform(shadow_detail.directional_shadow_resolution() as f32);
        let point_shadow_size = ScreenSize::uniform(shadow_detail.point_shadow_resolution() as f32);

        let solid_pixel_texture = Arc::new(Texture::new_with_data(
            device,
            queue,
            &TextureDescriptor {
                label: Some("solid pixel"),
                size: Extent3d::default(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: Default::default(),
            },
            &[255, 255, 255, 255],
        ));
        let walk_indicator_texture = texture_loader.get("grid.tga").unwrap();
        let screen_textures = Self::create_screen_size_textures(device, screen_size);
        let directional_shadow_map_texture = Self::create_directional_shadow_texture(device, directional_shadow_size);
        let point_shadow_map_textures = Self::create_point_shadow_textures(device, point_shadow_size);

        let picker_value_buffer = Buffer::with_capacity(
            device,
            "picker value",
            BufferUsages::STORAGE | BufferUsages::MAP_READ,
            PickerTarget::value_size() as _,
        );

        let global_uniforms_buffer = Buffer::with_capacity(
            device,
            "global uniforms",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<GlobalUniforms>() as _,
        );

        #[cfg(feature = "debug")]
        let debug_uniforms_buffer = Buffer::with_capacity(
            device,
            "debug uniforms",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<DebugUniforms>() as _,
        );

        let nearest_sampler = create_new_sampler(device, "nearest", TextureSamplerType::Nearest);
        let linear_sampler = create_new_sampler(device, "linear", TextureSamplerType::Linear);
        let texture_sampler = create_new_sampler(device, "texture", texture_sampler);

        let global_bind_group = Self::create_global_bind_group(
            device,
            &global_uniforms_buffer,
            #[cfg(feature = "debug")]
            &debug_uniforms_buffer,
            &nearest_sampler,
            &linear_sampler,
            &texture_sampler,
        );

        let picker_bind_group = Self::create_picker_bind_group(device, &screen_textures.picker_buffer_texture, &picker_value_buffer);
        let screen_bind_group = Self::create_screen_bind_group(
            device,
            &screen_textures.diffuse_buffer_texture,
            &screen_textures.normal_buffer_texture,
            &screen_textures.water_buffer_texture,
            &screen_textures.depth_buffer_texture,
            &directional_shadow_map_texture,
            &point_shadow_map_textures,
            &screen_textures.interface_buffer_texture,
            #[cfg(feature = "debug")]
            &screen_textures.picker_buffer_texture,
        );

        Self {
            surface_texture_format,
            solid_pixel_texture,
            walk_indicator_texture,
            depth_texture: screen_textures.depth_texture,
            picker_buffer_texture: screen_textures.picker_buffer_texture,
            diffuse_buffer_texture: screen_textures.diffuse_buffer_texture,
            normal_buffer_texture: screen_textures.normal_buffer_texture,
            water_buffer_texture: screen_textures.water_buffer_texture,
            depth_buffer_texture: screen_textures.depth_buffer_texture,
            interface_buffer_texture: screen_textures.interface_buffer_texture,
            directional_shadow_map_texture,
            point_shadow_map_textures,
            global_uniforms_buffer,
            #[cfg(feature = "debug")]
            debug_uniforms_buffer,
            picker_value_buffer,
            nearest_sampler,
            linear_sampler,
            texture_sampler,
            global_bind_group,
            picker_bind_group,
            screen_bind_group,
            screen_size,
            directional_shadow_size,
            point_shadow_size,
            global_uniforms: GlobalUniforms::default(),
            #[cfg(feature = "debug")]
            debug_uniforms: DebugUniforms::default(),
        }
    }

    fn create_screen_size_textures(device: &Device, screen_size: ScreenSize) -> ScreenSizeTextures {
        let screen_factory = AttachmentTextureFactory::new(device, screen_size, 1);

        let depth_texture = screen_factory.new_attachment("depth", TextureFormat::Depth32Float, AttachmentTextureType::Depth);
        let picker_buffer_texture =
            screen_factory.new_attachment("picker buffer", TextureFormat::Rg32Uint, AttachmentTextureType::ColorAttachment);

        let multisampled_screen_factory = AttachmentTextureFactory::new(device, screen_size, 4);

        let diffuse_buffer_texture = multisampled_screen_factory.new_attachment(
            "diffuse buffer",
            TextureFormat::Rgba8UnormSrgb,
            AttachmentTextureType::ColorAttachment,
        );
        let normal_buffer_texture = multisampled_screen_factory.new_attachment(
            "normal buffer",
            TextureFormat::Rgba16Float,
            AttachmentTextureType::ColorAttachment,
        );
        let water_buffer_texture = multisampled_screen_factory.new_attachment(
            "water buffer",
            TextureFormat::Rgba8UnormSrgb,
            AttachmentTextureType::ColorAttachment,
        );
        let depth_buffer_texture = multisampled_screen_factory.new_attachment(
            "depth buffer",
            TextureFormat::Depth32Float,
            AttachmentTextureType::DepthAttachment,
        );
        let interface_buffer_texture = multisampled_screen_factory.new_attachment(
            "interface buffer",
            TextureFormat::Rgba8UnormSrgb,
            AttachmentTextureType::ColorAttachment,
        );

        ScreenSizeTextures {
            depth_texture,
            picker_buffer_texture,
            diffuse_buffer_texture,
            normal_buffer_texture,
            water_buffer_texture,
            depth_buffer_texture,
            interface_buffer_texture,
        }
    }

    fn create_directional_shadow_texture(device: &Device, shadow_size: ScreenSize) -> AttachmentTexture {
        let shadow_factory = AttachmentTextureFactory::new(device, shadow_size, 1);

        shadow_factory.new_attachment(
            "directional shadow map",
            TextureFormat::Depth32Float,
            AttachmentTextureType::DepthAttachment,
        )
    }

    fn create_point_shadow_textures(device: &Device, shadow_size: ScreenSize) -> [CubeTexture; 6] {
        [
            CubeTexture::new(
                device,
                "point shadow map 0",
                shadow_size,
                TextureFormat::Depth32Float,
                AttachmentTextureType::DepthAttachment,
            ),
            CubeTexture::new(
                device,
                "point shadow map 1",
                shadow_size,
                TextureFormat::Depth32Float,
                AttachmentTextureType::DepthAttachment,
            ),
            CubeTexture::new(
                device,
                "point shadow map 2",
                shadow_size,
                TextureFormat::Depth32Float,
                AttachmentTextureType::DepthAttachment,
            ),
            CubeTexture::new(
                device,
                "point shadow map 3",
                shadow_size,
                TextureFormat::Depth32Float,
                AttachmentTextureType::DepthAttachment,
            ),
            CubeTexture::new(
                device,
                "point shadow map 4",
                shadow_size,
                TextureFormat::Depth32Float,
                AttachmentTextureType::DepthAttachment,
            ),
            CubeTexture::new(
                device,
                "point shadow map 5",
                shadow_size,
                TextureFormat::Depth32Float,
                AttachmentTextureType::DepthAttachment,
            ),
        ]
    }

    fn update_screen_size_textures(&mut self, device: &Device, screen_size: ScreenSize) {
        self.screen_size = screen_size;
        let new_textures = Self::create_screen_size_textures(device, self.screen_size);

        self.depth_texture = new_textures.depth_texture;
        self.picker_buffer_texture = new_textures.picker_buffer_texture;
        self.diffuse_buffer_texture = new_textures.diffuse_buffer_texture;
        self.normal_buffer_texture = new_textures.normal_buffer_texture;
        self.water_buffer_texture = new_textures.water_buffer_texture;
        self.depth_buffer_texture = new_textures.depth_buffer_texture;
        self.interface_buffer_texture = new_textures.interface_buffer_texture;

        // We need to update these bind groups, because their content changed, and they
        // are not re-created each frame.
        self.picker_bind_group = Self::create_picker_bind_group(device, &self.picker_buffer_texture, &self.picker_value_buffer);
        self.screen_bind_group = Self::create_screen_bind_group(
            device,
            &self.diffuse_buffer_texture,
            &self.normal_buffer_texture,
            &self.water_buffer_texture,
            &self.depth_buffer_texture,
            &self.directional_shadow_map_texture,
            &self.point_shadow_map_textures,
            &self.interface_buffer_texture,
            #[cfg(feature = "debug")]
            &self.picker_buffer_texture,
        );
    }

    fn update_shadow_size_textures(&mut self, device: &Device, shadow_detail: ShadowDetail) {
        self.directional_shadow_size = ScreenSize::uniform(shadow_detail.directional_shadow_resolution() as f32);
        self.point_shadow_size = ScreenSize::uniform(shadow_detail.point_shadow_resolution() as f32);

        self.directional_shadow_map_texture = Self::create_directional_shadow_texture(device, self.directional_shadow_size);
        self.point_shadow_map_textures = Self::create_point_shadow_textures(device, self.point_shadow_size);

        // We need to update this bind group, because it's content changed, and it isn't
        // re-created each frame.
        self.screen_bind_group = Self::create_screen_bind_group(
            device,
            &self.diffuse_buffer_texture,
            &self.normal_buffer_texture,
            &self.water_buffer_texture,
            &self.depth_buffer_texture,
            &self.directional_shadow_map_texture,
            &self.point_shadow_map_textures,
            &self.interface_buffer_texture,
            #[cfg(feature = "debug")]
            &self.picker_buffer_texture,
        );
    }

    fn update_texture_sampler(&mut self, device: &Device, texture_sampler_type: TextureSamplerType) {
        self.texture_sampler = create_new_sampler(device, "texture", texture_sampler_type);
        self.global_bind_group = Self::create_global_bind_group(
            device,
            &self.global_uniforms_buffer,
            #[cfg(feature = "debug")]
            &self.debug_uniforms_buffer,
            &self.nearest_sampler,
            &self.linear_sampler,
            &self.texture_sampler,
        );
    }

    fn global_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("global"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<GlobalUniforms>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                    #[cfg(feature = "debug")]
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<DebugUniforms>() as _),
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    fn picker_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("picker"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(PickerTarget::value_size() as _),
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    fn screen_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("screen"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS as _),
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                    #[cfg(feature = "debug")]
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    fn create_global_bind_group(
        device: &Device,
        global_uniforms_buffer: &Buffer<GlobalUniforms>,
        #[cfg(feature = "debug")] debug_uniforms_buffer: &Buffer<DebugUniforms>,
        nearest_sampler: &Sampler,
        linear_sampler: &Sampler,
        texture_sampler: &Sampler,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("global"),
            layout: Self::global_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: global_uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(nearest_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(linear_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(texture_sampler),
                },
                #[cfg(feature = "debug")]
                BindGroupEntry {
                    binding: 4,
                    resource: debug_uniforms_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_picker_bind_group(device: &Device, picker_texture: &AttachmentTexture, picker_buffer: &Buffer<u64>) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("picker"),
            layout: Self::picker_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(picker_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: picker_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_screen_bind_group(
        device: &Device,
        diffuse_buffer_texture: &AttachmentTexture,
        normal_buffer_texture: &AttachmentTexture,
        water_buffer_texture: &AttachmentTexture,
        depth_buffer_texture: &AttachmentTexture,
        directional_shadow_map_texture: &AttachmentTexture,
        point_shadow_maps_texture: &[CubeTexture; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS],
        interface_buffer_texture: &AttachmentTexture,
        #[cfg(feature = "debug")] picker_buffer_texture: &AttachmentTexture,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("screen"),
            layout: Self::screen_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(diffuse_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(normal_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(water_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(depth_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(directional_shadow_map_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureViewArray(&[
                        point_shadow_maps_texture[0].get_texture_view(),
                        point_shadow_maps_texture[1].get_texture_view(),
                        point_shadow_maps_texture[2].get_texture_view(),
                        point_shadow_maps_texture[3].get_texture_view(),
                        point_shadow_maps_texture[4].get_texture_view(),
                        point_shadow_maps_texture[5].get_texture_view(),
                    ]),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(interface_buffer_texture.get_texture_view()),
                },
                #[cfg(feature = "debug")]
                BindGroupEntry {
                    binding: 7,
                    resource: BindingResource::TextureView(picker_buffer_texture.get_texture_view()),
                },
            ],
        })
    }
}

struct ScreenSizeTextures {
    depth_texture: AttachmentTexture,
    picker_buffer_texture: AttachmentTexture,
    diffuse_buffer_texture: AttachmentTexture,
    normal_buffer_texture: AttachmentTexture,
    water_buffer_texture: AttachmentTexture,
    depth_buffer_texture: AttachmentTexture,
    interface_buffer_texture: AttachmentTexture,
}
