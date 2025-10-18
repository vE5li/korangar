mod buffer;
mod capabilities;
mod color;
mod engine;
#[cfg(feature = "debug")]
mod error;
mod frame_pacer;
mod instruction;
mod passes;
mod picker_target;
mod primitives;
mod projection;
mod sampler;
mod settings;
mod shader_compiler;
mod surface;
mod texture;
mod vertices;

use std::num::NonZeroU64;
use std::sync::{Arc, OnceLock};

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, SquareMatrix, Vector4, Zero};
use image::RgbaImage;
use wgpu::util::StagingBelt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, CommandEncoder, Device, Extent3d, Queue, Sampler,
    SamplerBindingType, ShaderStages, StorageTextureAccess, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureViewDimension,
};

pub use self::buffer::Buffer;
pub use self::capabilities::*;
pub use self::color::*;
pub use self::engine::{GraphicsEngine, GraphicsEngineDescriptor};
#[cfg(feature = "debug")]
pub use self::error::error_handler;
pub use self::frame_pacer::*;
pub use self::instruction::*;
pub use self::passes::{Lanczos3Drawer, MipMapRenderPassContext};
pub use self::picker_target::PickerTarget;
pub use self::primitives::*;
pub use self::projection::*;
pub use self::settings::*;
pub use self::shader_compiler::ShaderCompiler;
pub use self::surface::*;
pub use self::texture::*;
pub use self::vertices::*;
use crate::NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS;
use crate::graphics::sampler::{SamplerType, create_new_sampler};
use crate::loaders::{ImageType, TextureLoader};

/// The size of a tile in pixel of the tile based light culling.
const LIGHT_TILE_SIZE: u32 = 16;

/// The count of shadow maps in which we partition the directional shadow.
/// We can't make this an overridable constant in WGSL or runtime defined, since
/// we use the const in the WGSL shaders in locations that need const and don't
/// allow overrides. So if you change this variable, you also need to replace
/// the constants in the shaders of the same name.
pub const PARTITION_COUNT: usize = 3;

pub const RENDER_TO_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
pub const RENDER_TO_TEXTURE_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
pub const INTERFACE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
pub const FXAA_COLOR_LUMA_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

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
    view: [[f32; 4]; 4],
    inverse_view: [[f32; 4]; 4],
    inverse_projection: [[f32; 4]; 4],
    inverse_view_projection: [[f32; 4]; 4],
    indicator_positions: [[f32; 4]; 4],
    indicator_color: [f32; 4],
    ambient_color: [f32; 4],
    camera_position: [f32; 4],
    forward_size: [u32; 2],
    interface_size: [u32; 2],
    pointer_position: [u32; 2],
    animation_timer: f32,
    point_light_count: u32,
    enhanced_lighting: u32,
    shadow_method: u32,
    shadow_detail: u32,
    use_sdsm: u32,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct KernelUniforms {
    // Packed as vec4 for std140 layout: vec4(point1.xy, point2.xy)
    sample_points_8: [[f32; 4]; 4],
    sample_points_16: [[f32; 4]; 8],
    sample_points_32: [[f32; 4]; 16],
    sample_points_64: [[f32; 4]; 32],
}

impl KernelUniforms {
    const fn pack_vec2_to_vec4<const INPUT_LEN: usize, const OUTPUT_LEN: usize>(points: &[[f32; 2]; INPUT_LEN]) -> [[f32; 4]; OUTPUT_LEN] {
        let mut packed = [[0.0f32; 4]; OUTPUT_LEN];

        let mut i = 0;
        while i < OUTPUT_LEN {
            packed[i] = [points[i * 2][0], points[i * 2][1], points[i * 2 + 1][0], points[i * 2 + 1][1]];
            i += 1;
        }

        packed
    }

    const fn initialize() -> Self {
        let points_8 = [
            [0.125, -0.375],
            [-0.125, 0.375],
            [0.625, 0.125],
            [-0.375, -0.625],
            [-0.625, 0.625],
            [-0.875, -0.125],
            [0.375, 0.875],
            [0.875, -0.875],
        ];

        let points_16 = [
            [-0.875, -0.875],
            [-0.750, -0.125],
            [-0.625, 0.625],
            [-0.500, -0.375],
            [-0.375, 0.875],
            [-0.250, 0.125],
            [-0.125, -0.625],
            [0.000, 0.375],
            [0.125, -0.750],
            [0.250, 0.500],
            [0.375, -0.250],
            [0.500, 0.750],
            [0.625, 0.000],
            [0.750, -0.500],
            [0.875, 0.250],
            [1.000, -1.000],
        ];

        let points_32 = [
            [0.06407013, 0.05409927],
            [0.7366577, 0.5789394],
            [-0.6270542, -0.5320278],
            [-0.4096107, 0.8411095],
            [0.6849564, -0.4990818],
            [-0.874181, -0.04579735],
            [0.9989998, 0.0009880066],
            [-0.004920578, -0.9151649],
            [0.1805763, 0.9747483],
            [-0.2138451, 0.2635818],
            [0.109845, 0.3884785],
            [0.06876755, -0.3581074],
            [0.374073, -0.7661266],
            [0.3079132, -0.1216763],
            [-0.3794335, -0.8271583],
            [-0.203878, -0.07715034],
            [0.5912697, 0.1469799],
            [-0.88069, 0.3031784],
            [0.5040108, 0.8283722],
            [-0.5844124, 0.5494877],
            [0.6017799, -0.1726654],
            [-0.5554981, 0.1559997],
            [-0.3016369, -0.3900928],
            [-0.5550632, -0.1723762],
            [0.925029, 0.2995041],
            [-0.2473137, 0.5538505],
            [0.9183037, -0.2862392],
            [0.2469421, 0.6718712],
            [0.3916397, -0.4328209],
            [-0.03576927, -0.6220032],
            [-0.04661255, 0.7995201],
            [0.4402924, 0.3640312],
        ];

        let points_64 = [
            [-0.5119625, -0.4827938],
            [-0.2171264, -0.4768726],
            [-0.7552931, -0.2426507],
            [-0.7136765, -0.4496614],
            [-0.5938849, -0.6895654],
            [-0.3148003, -0.7047654],
            [-0.42215, -0.2024607],
            [-0.9466816, -0.2014508],
            [-0.8409063, -0.03465778],
            [-0.6517572, -0.07476326],
            [-0.1041822, -0.02521214],
            [-0.3042712, -0.02195431],
            [-0.5082307, 0.1079806],
            [-0.08429877, -0.2316298],
            [-0.9879128, 0.1113683],
            [-0.3859636, 0.3363545],
            [-0.1925334, 0.1787288],
            [0.003256182, 0.138135],
            [-0.8706837, 0.3010679],
            [-0.6982038, 0.1904326],
            [0.1975043, 0.2221317],
            [0.1507788, 0.4204168],
            [0.3514056, 0.09865579],
            [0.1558783, -0.08460935],
            [-0.0684978, 0.4461993],
            [0.3780522, 0.3478679],
            [0.3956799, -0.1469177],
            [0.5838975, 0.1054943],
            [0.6155105, 0.3245716],
            [0.3928624, -0.4417621],
            [0.1749884, -0.4202175],
            [0.6813727, -0.2424808],
            [-0.6707711, 0.4912741],
            [0.0005130528, -0.8058334],
            [0.02703013, -0.6010728],
            [-0.1658188, -0.9695674],
            [0.4060591, -0.7100726],
            [0.7713396, -0.4713659],
            [0.573212, -0.51544],
            [-0.3448896, -0.9046497],
            [0.1268544, -0.9874692],
            [0.7418533, -0.6667366],
            [0.3492522, 0.5924662],
            [0.5679897, 0.5343465],
            [0.5663417, 0.7708698],
            [0.7375497, 0.6691415],
            [0.2271994, -0.6163502],
            [0.2312844, 0.8725659],
            [0.4216993, 0.9002838],
            [0.4262091, -0.9013284],
            [0.2001408, -0.808381],
            [0.149394, 0.6650763],
            [-0.09640376, 0.9843736],
            [0.7682328, -0.07273844],
            [0.04146584, 0.8313184],
            [0.9705266, -0.1143304],
            [0.9670017, 0.1293385],
            [0.9015037, -0.3306949],
            [-0.5085648, 0.7534177],
            [0.9055501, 0.3758393],
            [0.7599946, 0.1809109],
            [-0.2483695, 0.7942952],
            [-0.4241052, 0.5581087],
            [-0.1020106, 0.6724468],
        ];

        Self {
            sample_points_8: Self::pack_vec2_to_vec4(&points_8),
            sample_points_16: Self::pack_vec2_to_vec4(&points_16),
            sample_points_32: Self::pack_vec2_to_vec4(&points_32),
            sample_points_64: Self::pack_vec2_to_vec4(&points_64),
        }
    }
}

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct DirectionalLightUniforms {
    view_projection: [[f32; 4]; 4],
    color: [f32; 4],
    direction: [f32; 4],
}

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct DirectionalLightPartition {
    view_projection: [[f32; 4]; 4],
    interval_end: f32,
    world_space_texel_size: f32,
    near_plane: f32,
    far_plane: f32,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct PointLightData {
    position: [f32; 4],
    color: [f32; 4],
    range: f32,
    texture_index: i32,
    padding: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct Partition {
    extents: [f32; 4],
    center: [f32; 4],
    interval_begin: f32,
    interval_end: f32,
    padding: [u32; 2],
}

#[derive(Copy, Clone, Debug)]
pub struct DirectionalShadowPartition {
    pub extents: Vector4<f32>,
    pub center: Vector4<f32>,
    pub interval_end: f32,
}

impl Default for DirectionalShadowPartition {
    fn default() -> Self {
        Self {
            extents: Vector4::zero(),
            center: Vector4::zero(),
            interval_end: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct Interval {
    begin: u32,
    end: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct Bounds {
    min_coord_x: u32,
    min_coord_y: u32,
    min_coord_z: u32,
    max_coord_x: u32,
    max_coord_y: u32,
    max_coord_z: u32,
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct DebugUniforms {
    show_picker_buffer: u32,
    show_directional_shadow_map: u32,
    show_point_shadow_map: u32,
    show_light_culling_count_buffer: u32,
    show_sdsm_partitions: u32,
    show_font_map: u32,
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct TileLightIndices {
    indices: [u32; 256],
}

/// Holds all GPU resources that are shared by multiple passes.
pub(crate) struct GlobalContext {
    pub(crate) surface_texture_format: TextureFormat,
    pub(crate) msaa: Msaa,
    pub(crate) ssaa: Ssaa,
    pub(crate) screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
    pub(crate) high_quality_interface: bool,
    pub(crate) solid_pixel_texture: Arc<Texture>,
    pub(crate) walk_indicator_texture: Arc<Texture>,
    pub(crate) forward_depth_texture: AttachmentTexture,
    pub(crate) picker_buffer_texture: AttachmentTexture,
    pub(crate) picker_depth_texture: AttachmentTexture,
    pub(crate) forward_color_texture: AttachmentTexture,
    pub(crate) forward_accumulation_texture: AttachmentTexture,
    pub(crate) forward_revealage_texture: AttachmentTexture,
    pub(crate) resolved_color_texture: Option<AttachmentTexture>,
    pub(crate) supersampled_color_texture: Option<AttachmentTexture>,
    pub(crate) interface_buffer_texture: AttachmentTexture,
    pub(crate) directional_shadow_map_texture: AttachmentTexture,
    pub(crate) directional_shadow_translucence_texture: AttachmentTexture,
    pub(crate) point_shadow_map_textures: CubeArrayTexture,
    pub(crate) tile_light_count_texture: StorageTexture,
    pub(crate) global_uniforms_buffer: Buffer<GlobalUniforms>,
    pub(crate) kernel_uniforms_buffer: Buffer<KernelUniforms>,
    pub(crate) directional_light_uniforms_buffer: Buffer<DirectionalLightUniforms>,
    pub(crate) directional_light_partitions_buffer: Buffer<DirectionalLightPartition>,
    pub(crate) point_light_data_buffer: Buffer<PointLightData>,
    #[cfg(feature = "debug")]
    pub(crate) debug_uniforms_buffer: Buffer<DebugUniforms>,
    pub(crate) picker_value_buffer: Buffer<u64>,
    pub(crate) tile_light_indices_buffer: Buffer<TileLightIndices>,
    pub(crate) partition_data_buffer: Buffer<Partition>,
    pub(crate) partition_value_buffer: Buffer<Partition>,
    pub(crate) interval_data_buffer: Buffer<Interval>,
    pub(crate) bounds_data_buffer: Buffer<Bounds>,
    pub(crate) anti_aliasing_resources: AntiAliasingResources,
    pub(crate) nearest_sampler: Sampler,
    pub(crate) linear_sampler: Sampler,
    pub(crate) texture_sampler: Sampler,
    pub(crate) shadow_map_sampler: Sampler,
    pub(crate) global_bind_group: BindGroup,
    pub(crate) light_culling_bind_group: BindGroup,
    pub(crate) forward_bind_group: BindGroup,
    pub(crate) sdsm_bind_group: BindGroup,
    #[cfg(feature = "debug")]
    pub(crate) debug_bind_group: BindGroup,
    pub(crate) screen_size: ScreenSize,
    pub(crate) forward_size: ScreenSize,
    pub(crate) interface_size: ScreenSize,
    pub(crate) directional_shadow_size: ScreenSize,
    pub(crate) point_shadow_size: ScreenSize,
    global_uniforms: GlobalUniforms,
    directional_light_uniforms: DirectionalLightUniforms,
    directional_light_partitions_data: Vec<DirectionalLightPartition>,
    point_light_data: Vec<PointLightData>,
    #[cfg(feature = "debug")]
    debug_uniforms: DebugUniforms,
}

impl Prepare for GlobalContext {
    fn prepare(&mut self, _device: &Device, instructions: &RenderInstruction) {
        self.directional_light_partitions_data.clear();
        self.point_light_data.clear();

        #[allow(unused_mut)]
        let mut ambient_light_color = instructions.uniforms.ambient_light_color;

        #[cfg(feature = "debug")]
        if !instructions.render_options.enable_ambient_lighting {
            ambient_light_color = Color::BLACK;
        };

        #[allow(unused_mut)]
        let mut directional_light_color = instructions.directional_light.color;

        #[cfg(feature = "debug")]
        if !instructions.render_options.enable_directional_lighting {
            directional_light_color = Color::BLACK;
        };

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
            view: instructions.uniforms.view_matrix.into(),
            inverse_view: instructions.uniforms.view_matrix.invert().unwrap_or_else(Matrix4::identity).into(),
            inverse_projection: instructions
                .uniforms
                .projection_matrix
                .invert()
                .unwrap_or_else(Matrix4::identity)
                .into(),
            inverse_view_projection: view_projection.invert().unwrap_or_else(Matrix4::identity).into(),
            indicator_positions: indicator_positions.into(),
            indicator_color: indicator_color.components_linear(),
            ambient_color: ambient_light_color.components_linear(),
            camera_position: instructions.uniforms.camera_position.into(),
            forward_size: [self.forward_size.width as u32, self.forward_size.height as u32],
            interface_size: [self.interface_size.width as u32, self.interface_size.height as u32],
            pointer_position: [instructions.picker_position.left as u32, instructions.picker_position.top as u32],
            animation_timer: instructions.uniforms.animation_timer_ms / 1000.0,
            point_light_count: (instructions.point_light_with_shadows.len() + instructions.point_light.len()) as u32,
            enhanced_lighting: instructions.uniforms.enhanced_lighting as u32,
            shadow_method: instructions.uniforms.shadow_method.into(),
            shadow_detail: instructions.uniforms.shadow_detail.into(),
            use_sdsm: instructions.uniforms.use_sdsm as u32,
        };

        self.directional_light_uniforms = DirectionalLightUniforms {
            view_projection: instructions.directional_light.view_projection_matrix.into(),
            color: directional_light_color.components_linear(),
            direction: instructions.directional_light.direction.extend(0.0).into(),
        };

        for partition in instructions.directional_light_partitions {
            self.directional_light_partitions_data.push(DirectionalLightPartition {
                view_projection: partition.view_projection_matrix.into(),
                interval_end: partition.interval_end,
                world_space_texel_size: partition.world_space_texel_size,
                near_plane: partition.near_plane,
                far_plane: partition.far_plane,
            });
        }

        for (instance_index, instruction) in instructions.point_light_with_shadows.iter().enumerate() {
            self.point_light_data.push(PointLightData {
                position: instruction.position.to_homogeneous().into(),
                color: instruction.color.components_linear(),
                range: instruction.range,
                texture_index: (instance_index + 1) as i32,
                padding: Default::default(),
            });
        }

        for instruction in instructions.point_light.iter() {
            self.point_light_data.push(PointLightData {
                position: instruction.position.to_homogeneous().into(),
                color: instruction.color.components_linear(),
                range: instruction.range,
                texture_index: 0,
                padding: Default::default(),
            });
        }

        #[cfg(feature = "debug")]
        {
            self.debug_uniforms = DebugUniforms {
                show_picker_buffer: instructions.render_options.show_picker_buffer as u32,
                show_directional_shadow_map: instructions
                    .render_options
                    .show_directional_shadow_map
                    .map(|value| value.get())
                    .unwrap_or(0),
                show_point_shadow_map: instructions
                    .render_options
                    .show_point_shadow_map
                    .map(|value| value.get())
                    .unwrap_or(0),
                show_light_culling_count_buffer: instructions.render_options.show_light_culling_count_buffer as u32,
                show_sdsm_partitions: instructions.render_options.show_sdsm_partitions as u32,
                show_font_map: instructions.render_options.show_font_map as u32,
            };
        }
    }

    fn upload(&mut self, device: &Device, staging_belt: &mut StagingBelt, command_encoder: &mut CommandEncoder) {
        let mut recreated = self
            .global_uniforms_buffer
            .write(device, staging_belt, command_encoder, &[self.global_uniforms]);

        recreated |=
            self.directional_light_uniforms_buffer
                .write(device, staging_belt, command_encoder, &[self.directional_light_uniforms]);

        if !self.point_light_data.is_empty() {
            recreated |= self
                .point_light_data_buffer
                .write(device, staging_belt, command_encoder, &self.point_light_data);
        }

        recreated |=
            self.directional_light_partitions_buffer
                .write(device, staging_belt, command_encoder, &self.directional_light_partitions_data);

        #[cfg(feature = "debug")]
        {
            recreated |= self
                .debug_uniforms_buffer
                .write(device, staging_belt, command_encoder, &[self.debug_uniforms]);
        }

        if recreated {
            self.global_bind_group = Self::create_global_bind_group(
                device,
                &self.global_uniforms_buffer,
                &self.nearest_sampler,
                &self.linear_sampler,
                &self.texture_sampler,
                &self.shadow_map_sampler,
            );

            self.light_culling_bind_group = Self::create_light_culling_bind_group(
                device,
                &self.point_light_data_buffer,
                &self.tile_light_count_texture,
                &self.tile_light_indices_buffer,
            );

            self.forward_bind_group = Self::create_forward_bind_group(
                device,
                &self.directional_light_uniforms_buffer,
                &self.point_light_data_buffer,
                &self.tile_light_count_texture,
                &self.tile_light_indices_buffer,
                &self.directional_shadow_map_texture,
                &self.directional_shadow_translucence_texture,
                &self.point_shadow_map_textures,
                &self.directional_light_partitions_buffer,
                &self.kernel_uniforms_buffer,
            );

            self.sdsm_bind_group = Self::create_sdsm_bind_group(
                device,
                self.msaa,
                &self.directional_light_uniforms_buffer,
                &self.forward_depth_texture,
                &self.partition_data_buffer,
                &self.interval_data_buffer,
                &self.bounds_data_buffer,
            );

            #[cfg(feature = "debug")]
            {
                self.debug_bind_group = Self::create_debug_bind_group(
                    device,
                    self.msaa,
                    &self.debug_uniforms_buffer,
                    &self.picker_buffer_texture,
                    &self.directional_shadow_map_texture,
                    &self.tile_light_count_texture,
                    &self.point_shadow_map_textures,
                    &self.forward_depth_texture,
                    &self.partition_data_buffer,
                );
            }
        }
    }
}

impl GlobalContext {
    fn new(
        device: &Device,
        queue: &Queue,
        capabilities: &Capabilities,
        texture_loader: &TextureLoader,
        surface_texture_format: TextureFormat,
        msaa: Msaa,
        ssaa: Ssaa,
        screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
        screen_size: ScreenSize,
        shadow_resolution: ShadowResolution,
        texture_sampler: TextureSamplerType,
        high_quality_interface: bool,
    ) -> Self {
        let forward_size = ssaa.calculate_size(screen_size);
        let interface_size = if high_quality_interface { screen_size * 2.0 } else { screen_size };
        let directional_shadow_size = ScreenSize::uniform(shadow_resolution.directional_shadow_resolution() as f32);
        let point_shadow_size = ScreenSize::uniform(shadow_resolution.point_shadow_resolution() as f32);

        let solid_pixel_texture = Arc::new(Texture::new_with_data(
            device,
            queue,
            &TextureDescriptor {
                label: Some("solid pixel"),
                size: Extent3d::default(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: Default::default(),
            },
            RgbaImage::from_raw(1, 1, vec![255, 255, 255, 255]).unwrap().as_raw(),
            false,
        ));
        let walk_indicator_texture = texture_loader.get_or_load("grid.tga", ImageType::Color).unwrap();
        let forward_textures = Self::create_forward_textures(device, forward_size, msaa);
        let picker_textures = Self::create_picker_textures(device, screen_size);
        let directional_shadow_map_texture = Self::create_directional_shadow_textures(device, directional_shadow_size);
        let directional_shadow_translucence_texture =
            Self::create_directional_shadow_translucence_textures(device, directional_shadow_size);
        let point_shadow_map_textures = Self::create_point_shadow_textures(device, point_shadow_size);
        let resolved_color_texture = Self::create_resolved_color_texture(device, forward_size, msaa);
        let supersampled_color_texture = Self::create_supersampled_texture(device, screen_size, ssaa);
        let interface_buffer_texture = Self::create_interface_texture(device, interface_size);

        let picker_value_buffer = Buffer::with_capacity(
            device,
            "picker value",
            BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            PickerTarget::value_size() as _,
        );

        let global_uniforms_buffer = Buffer::with_capacity(
            device,
            "global uniforms",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<GlobalUniforms>() as _,
        );

        let kernel_uniforms_buffer = Buffer::with_capacity(
            device,
            "kernel uniforms",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<KernelUniforms>() as _,
        );
        // The data is static and only needs to be uploaded once.
        kernel_uniforms_buffer.write_exact(queue, &[KernelUniforms::initialize()]);

        let directional_light_uniforms_buffer = Buffer::with_capacity(
            device,
            "directional light uniforms",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<DirectionalLightUniforms>() as _,
        );

        #[cfg(feature = "debug")]
        let debug_uniforms_buffer = Buffer::with_capacity(
            device,
            "debug uniforms",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<DebugUniforms>() as _,
        );

        let directional_light_partitions_buffer = Buffer::with_capacity(
            device,
            "directional light partitions",
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (PARTITION_COUNT * size_of::<DirectionalLightPartition>()) as _,
        );

        let point_light_data_buffer = Buffer::with_capacity(
            device,
            "point light data",
            BufferUsages::COPY_DST | BufferUsages::STORAGE,
            (128 * size_of::<PointLightData>()) as _,
        );

        let tile_light_indices_buffer = Self::create_tile_light_indices_buffer(device, forward_size);

        let partition_data_buffer = Buffer::with_capacity(
            device,
            "partition data",
            BufferUsages::COPY_SRC | BufferUsages::STORAGE,
            (PARTITION_COUNT * size_of::<Partition>()) as _,
        );

        let partition_value_buffer = Buffer::with_capacity(
            device,
            "partition value",
            BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            (PARTITION_COUNT * size_of::<Partition>()) as _,
        );

        let bounds_data_buffer = Buffer::with_capacity(
            device,
            "bounds data",
            BufferUsages::COPY_SRC | BufferUsages::STORAGE,
            (PARTITION_COUNT * size_of::<Bounds>()) as _,
        );

        let interval_data_buffer = Buffer::with_capacity(
            device,
            "interval data",
            BufferUsages::STORAGE,
            (PARTITION_COUNT * size_of::<Interval>()) as _,
        );

        let nearest_sampler = create_new_sampler(device, capabilities, "nearest", SamplerType::TextureNearest);
        let linear_sampler = create_new_sampler(device, capabilities, "linear", SamplerType::TextureLinear);
        let texture_sampler = create_new_sampler(device, capabilities, "texture", texture_sampler);
        let shadow_map_sampler = create_new_sampler(device, capabilities, "shadow map", SamplerType::DepthCompare);

        let anti_aliasing_resources = Self::create_anti_aliasing_resources(device, screen_space_anti_aliasing, screen_size);

        let global_bind_group = Self::create_global_bind_group(
            device,
            &global_uniforms_buffer,
            &nearest_sampler,
            &linear_sampler,
            &texture_sampler,
            &shadow_map_sampler,
        );

        let light_culling_bind_group = Self::create_light_culling_bind_group(
            device,
            &point_light_data_buffer,
            &forward_textures.tile_light_count_texture,
            &tile_light_indices_buffer,
        );

        let forward_bind_group = Self::create_forward_bind_group(
            device,
            &directional_light_uniforms_buffer,
            &point_light_data_buffer,
            &forward_textures.tile_light_count_texture,
            &tile_light_indices_buffer,
            &directional_shadow_map_texture,
            &directional_shadow_translucence_texture,
            &point_shadow_map_textures,
            &directional_light_partitions_buffer,
            &kernel_uniforms_buffer,
        );

        let sdsm_bind_group = Self::create_sdsm_bind_group(
            device,
            msaa,
            &directional_light_uniforms_buffer,
            &forward_textures.forward_depth_texture,
            &partition_data_buffer,
            &interval_data_buffer,
            &bounds_data_buffer,
        );

        #[cfg(feature = "debug")]
        let debug_bind_group = Self::create_debug_bind_group(
            device,
            msaa,
            &debug_uniforms_buffer,
            &picker_textures.picker_buffer_texture,
            &directional_shadow_map_texture,
            &forward_textures.tile_light_count_texture,
            &point_shadow_map_textures,
            &forward_textures.forward_depth_texture,
            &partition_data_buffer,
        );

        Self {
            surface_texture_format,
            msaa,
            ssaa,
            screen_space_anti_aliasing,
            high_quality_interface,
            solid_pixel_texture,
            walk_indicator_texture,
            forward_depth_texture: forward_textures.forward_depth_texture,
            picker_buffer_texture: picker_textures.picker_buffer_texture,
            picker_depth_texture: picker_textures.picker_depth_texture,
            forward_color_texture: forward_textures.forward_color_texture,
            forward_accumulation_texture: forward_textures.forward_accumulation_texture,
            forward_revealage_texture: forward_textures.forward_revealage_texture,
            resolved_color_texture,
            supersampled_color_texture,
            interface_buffer_texture,
            directional_shadow_map_texture,
            directional_shadow_translucence_texture,
            point_shadow_map_textures,
            tile_light_count_texture: forward_textures.tile_light_count_texture,
            global_uniforms_buffer,
            kernel_uniforms_buffer,
            forward_bind_group,
            sdsm_bind_group,
            #[cfg(feature = "debug")]
            debug_bind_group,
            directional_light_uniforms_buffer,
            tile_light_indices_buffer,
            partition_data_buffer,
            partition_value_buffer,
            bounds_data_buffer,
            #[cfg(feature = "debug")]
            debug_uniforms_buffer,
            picker_value_buffer,
            directional_light_partitions_buffer,
            point_light_data_buffer,
            anti_aliasing_resources,
            nearest_sampler,
            linear_sampler,
            texture_sampler,
            shadow_map_sampler,
            global_bind_group,
            light_culling_bind_group,
            screen_size,
            forward_size,
            interface_size,
            directional_shadow_size,
            point_shadow_size,
            global_uniforms: GlobalUniforms::default(),
            directional_light_uniforms: DirectionalLightUniforms::default(),
            directional_light_partitions_data: Vec::default(),
            point_light_data: Vec::default(),
            #[cfg(feature = "debug")]
            debug_uniforms: DebugUniforms::default(),
            interval_data_buffer,
        }
    }

    fn get_color_texture(&self) -> &AttachmentTexture {
        self.supersampled_color_texture
            .as_ref()
            .unwrap_or_else(|| self.resolved_color_texture.as_ref().unwrap_or(&self.forward_color_texture))
    }

    fn get_forward_texture(&self) -> &AttachmentTexture {
        self.resolved_color_texture.as_ref().unwrap_or(&self.forward_color_texture)
    }

    fn create_forward_textures(device: &Device, forward_size: ScreenSize, msaa: Msaa) -> ForwardTextures {
        let factory = AttachmentTextureFactory::new(device, forward_size, msaa.sample_count(), None);
        let forward_color_texture = factory.new_attachment(
            "forward color",
            RENDER_TO_TEXTURE_FORMAT,
            AttachmentTextureType::ColorAttachment,
        );
        let forward_depth_texture = factory.new_attachment("forward depth", RENDER_TO_TEXTURE_DEPTH_FORMAT, AttachmentTextureType::Depth);
        let forward_accumulation_texture = factory.new_attachment(
            "forward accumulation",
            TextureFormat::Rgba16Float,
            AttachmentTextureType::ColorAttachment,
        );
        let forward_revealage_texture = factory.new_attachment(
            "forward revealage",
            TextureFormat::R8Unorm,
            AttachmentTextureType::ColorAttachment,
        );

        let (tile_x, tile_y) = calculate_light_tile_count(forward_size);
        let tile_light_count_texture = StorageTexture::new(device, "tile light count texture", tile_x, tile_y, TextureFormat::R32Uint);

        ForwardTextures {
            forward_depth_texture,
            forward_color_texture,
            forward_accumulation_texture,
            forward_revealage_texture,
            tile_light_count_texture,
        }
    }

    fn create_picker_textures(device: &Device, screen_size: ScreenSize) -> PickerTextures {
        // Since we need to copy from the picker attachment to read the picker value, we
        // need to align both attachments properly to the requirements of
        // COPY_BYTES_PER_ROW_ALIGNMENT.
        let block_size = TextureFormat::Rg32Uint.block_copy_size(None).unwrap();
        let picker_padded_width = ((screen_size.width as u32 * block_size + (COPY_BYTES_PER_ROW_ALIGNMENT - 1))
            & !(COPY_BYTES_PER_ROW_ALIGNMENT - 1))
            / block_size;

        let picker_factory = AttachmentTextureFactory::new(device, screen_size, 1, Some(picker_padded_width));
        let picker_buffer_texture = picker_factory.new_attachment(
            "picker buffer",
            TextureFormat::Rg32Uint,
            AttachmentTextureType::PickerAttachment,
        );
        let picker_depth_texture = picker_factory.new_attachment("depth", RENDER_TO_TEXTURE_DEPTH_FORMAT, AttachmentTextureType::Depth);

        PickerTextures {
            picker_buffer_texture,
            picker_depth_texture,
        }
    }

    fn create_resolved_color_texture(device: &Device, forward_size: ScreenSize, msaa: Msaa) -> Option<AttachmentTexture> {
        match msaa.multisampling_activated() {
            true => {
                let attachment_factory = AttachmentTextureFactory::new(device, forward_size, 1, None);
                Some(attachment_factory.new_attachment(
                    "resolved color",
                    RENDER_TO_TEXTURE_FORMAT,
                    AttachmentTextureType::ColorAttachment,
                ))
            }
            false => None,
        }
    }

    fn create_supersampled_texture(device: &Device, screen_size: ScreenSize, ssaa: Ssaa) -> Option<AttachmentTexture> {
        match ssaa.supersampling_activated() {
            true => {
                let attachment_factory = AttachmentTextureFactory::new(device, screen_size, 1, None);
                Some(attachment_factory.new_attachment(
                    "supersampled color",
                    RENDER_TO_TEXTURE_FORMAT,
                    AttachmentTextureType::ColorAttachment,
                ))
            }
            false => None,
        }
    }

    fn create_interface_texture(device: &Device, interface_size: ScreenSize) -> AttachmentTexture {
        let interface_screen_factory = AttachmentTextureFactory::new(device, interface_size, 1, None);

        interface_screen_factory.new_attachment(
            "interface buffer",
            INTERFACE_TEXTURE_FORMAT,
            AttachmentTextureType::ColorAttachment,
        )
    }

    fn create_directional_shadow_textures(device: &Device, shadow_size: ScreenSize) -> AttachmentTexture {
        let shadow_factory = AttachmentTextureFactory::new(device, shadow_size, 1, None);

        shadow_factory.new_attachment_array(
            "directional shadow map",
            TextureFormat::Depth16Unorm,
            AttachmentTextureType::DepthAttachment,
            PARTITION_COUNT as u32,
        )
    }

    fn create_directional_shadow_translucence_textures(device: &Device, shadow_size: ScreenSize) -> AttachmentTexture {
        let shadow_factory = AttachmentTextureFactory::new(device, shadow_size, 1, None);

        shadow_factory.new_attachment_array(
            "directional shadow translucence",
            TextureFormat::R8Unorm,
            AttachmentTextureType::ColorAttachment,
            PARTITION_COUNT as u32,
        )
    }

    fn create_tile_light_indices_buffer(device: &Device, forward_size: ScreenSize) -> Buffer<TileLightIndices> {
        let (tile_count_x, tile_count_y) = calculate_light_tile_count(forward_size);

        Buffer::with_capacity(
            device,
            "tile light indices",
            BufferUsages::STORAGE,
            ((tile_count_x * tile_count_y).max(1) as usize * size_of::<TileLightIndices>()) as _,
        )
    }

    fn create_point_shadow_textures(device: &Device, shadow_size: ScreenSize) -> CubeArrayTexture {
        CubeArrayTexture::new(
            device,
            "point shadow map",
            shadow_size,
            TextureFormat::Depth32Float,
            AttachmentTextureType::DepthAttachment,
            NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS as u32,
        )
    }

    fn create_anti_aliasing_resources(
        device: &Device,
        screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
        screen_size: ScreenSize,
    ) -> AntiAliasingResources {
        match screen_space_anti_aliasing {
            ScreenSpaceAntiAliasing::Off => AntiAliasingResources::None,
            ScreenSpaceAntiAliasing::Fxaa => {
                let factory = AttachmentTextureFactory::new(device, screen_size, 1, None);
                let color_with_luma_texture = factory.new_attachment(
                    "fxaa2 color with luma",
                    FXAA_COLOR_LUMA_TEXTURE_FORMAT,
                    AttachmentTextureType::ColorAttachment,
                );
                let resources = FxaaResources { color_with_luma_texture };
                AntiAliasingResources::Fxaa(Box::new(resources))
            }
        }
    }

    fn update_screen_size_resources(&mut self, device: &Device, screen_size: ScreenSize) {
        self.screen_size = screen_size;
        self.forward_size = self.ssaa.calculate_size(self.screen_size);
        self.interface_size = if self.high_quality_interface {
            self.screen_size * 2.0
        } else {
            self.screen_size
        };

        let ForwardTextures {
            forward_color_texture,
            forward_depth_texture,
            forward_accumulation_texture,
            forward_revealage_texture,
            tile_light_count_texture,
        } = Self::create_forward_textures(device, self.forward_size, self.msaa);

        let PickerTextures {
            picker_buffer_texture,
            picker_depth_texture,
        } = Self::create_picker_textures(device, self.screen_size);

        let resolved_color_texture = Self::create_resolved_color_texture(device, self.forward_size, self.msaa);

        let supersampled_color_texture = Self::create_supersampled_texture(device, self.screen_size, self.ssaa);

        self.forward_color_texture = forward_color_texture;
        self.forward_depth_texture = forward_depth_texture;
        self.forward_accumulation_texture = forward_accumulation_texture;
        self.forward_revealage_texture = forward_revealage_texture;
        self.picker_buffer_texture = picker_buffer_texture;
        self.picker_depth_texture = picker_depth_texture;
        self.resolved_color_texture = resolved_color_texture;
        self.supersampled_color_texture = supersampled_color_texture;
        self.tile_light_count_texture = tile_light_count_texture;

        self.interface_buffer_texture = Self::create_interface_texture(device, self.interface_size);

        self.tile_light_indices_buffer = Self::create_tile_light_indices_buffer(device, self.forward_size);

        self.anti_aliasing_resources = Self::create_anti_aliasing_resources(device, self.screen_space_anti_aliasing, self.screen_size);

        // We need to update this bind group, because it's content changed, and it isn't
        // re-created each frame.
        self.light_culling_bind_group = Self::create_light_culling_bind_group(
            device,
            &self.point_light_data_buffer,
            &self.tile_light_count_texture,
            &self.tile_light_indices_buffer,
        );

        self.forward_bind_group = Self::create_forward_bind_group(
            device,
            &self.directional_light_uniforms_buffer,
            &self.point_light_data_buffer,
            &self.tile_light_count_texture,
            &self.tile_light_indices_buffer,
            &self.directional_shadow_map_texture,
            &self.directional_shadow_translucence_texture,
            &self.point_shadow_map_textures,
            &self.directional_light_partitions_buffer,
            &self.kernel_uniforms_buffer,
        );

        self.sdsm_bind_group = Self::create_sdsm_bind_group(
            device,
            self.msaa,
            &self.directional_light_uniforms_buffer,
            &self.forward_depth_texture,
            &self.partition_data_buffer,
            &self.interval_data_buffer,
            &self.bounds_data_buffer,
        );

        #[cfg(feature = "debug")]
        {
            self.debug_bind_group = Self::create_debug_bind_group(
                device,
                self.msaa,
                &self.debug_uniforms_buffer,
                &self.picker_buffer_texture,
                &self.directional_shadow_map_texture,
                &self.tile_light_count_texture,
                &self.point_shadow_map_textures,
                &self.forward_depth_texture,
                &self.partition_data_buffer,
            );
        }
    }

    fn update_shadow_size_textures(&mut self, device: &Device, shadow_resolution: ShadowResolution) {
        self.directional_shadow_size = ScreenSize::uniform(shadow_resolution.directional_shadow_resolution() as f32);
        self.point_shadow_size = ScreenSize::uniform(shadow_resolution.point_shadow_resolution() as f32);

        self.directional_shadow_map_texture = Self::create_directional_shadow_textures(device, self.directional_shadow_size);
        self.directional_shadow_translucence_texture =
            Self::create_directional_shadow_translucence_textures(device, self.directional_shadow_size);
        self.point_shadow_map_textures = Self::create_point_shadow_textures(device, self.point_shadow_size);

        // We need to update this bind group, because it's content changed, and it isn't
        // re-created each frame.
        self.forward_bind_group = Self::create_forward_bind_group(
            device,
            &self.directional_light_uniforms_buffer,
            &self.point_light_data_buffer,
            &self.tile_light_count_texture,
            &self.tile_light_indices_buffer,
            &self.directional_shadow_map_texture,
            &self.directional_shadow_translucence_texture,
            &self.point_shadow_map_textures,
            &self.directional_light_partitions_buffer,
            &self.kernel_uniforms_buffer,
        );

        #[cfg(feature = "debug")]
        {
            self.debug_bind_group = Self::create_debug_bind_group(
                device,
                self.msaa,
                &self.debug_uniforms_buffer,
                &self.picker_buffer_texture,
                &self.directional_shadow_map_texture,
                &self.tile_light_count_texture,
                &self.point_shadow_map_textures,
                &self.forward_depth_texture,
                &self.partition_data_buffer,
            );
        }
    }

    fn update_texture_sampler(&mut self, device: &Device, capabilities: &Capabilities, texture_sampler_type: TextureSamplerType) {
        self.texture_sampler = create_new_sampler(device, capabilities, "texture", texture_sampler_type);
        self.global_bind_group = Self::create_global_bind_group(
            device,
            &self.global_uniforms_buffer,
            &self.nearest_sampler,
            &self.linear_sampler,
            &self.texture_sampler,
            &self.shadow_map_sampler,
        );
    }

    fn update_msaa(&mut self, device: &Device, msaa: Msaa) {
        self.msaa = msaa;

        let ForwardTextures {
            forward_color_texture,
            forward_depth_texture,
            forward_accumulation_texture,
            forward_revealage_texture,
            tile_light_count_texture,
        } = Self::create_forward_textures(device, self.screen_size, self.msaa);

        self.forward_color_texture = forward_color_texture;
        self.forward_depth_texture = forward_depth_texture;
        self.forward_accumulation_texture = forward_accumulation_texture;
        self.forward_revealage_texture = forward_revealage_texture;
        self.tile_light_count_texture = tile_light_count_texture;
        self.resolved_color_texture = Self::create_resolved_color_texture(device, self.forward_size, self.msaa);

        self.sdsm_bind_group = Self::create_sdsm_bind_group(
            device,
            self.msaa,
            &self.directional_light_uniforms_buffer,
            &self.forward_depth_texture,
            &self.partition_data_buffer,
            &self.interval_data_buffer,
            &self.bounds_data_buffer,
        );

        #[cfg(feature = "debug")]
        {
            self.debug_bind_group = Self::create_debug_bind_group(
                device,
                self.msaa,
                &self.debug_uniforms_buffer,
                &self.picker_buffer_texture,
                &self.directional_shadow_map_texture,
                &self.tile_light_count_texture,
                &self.point_shadow_map_textures,
                &self.forward_depth_texture,
                &self.partition_data_buffer,
            );
        }
    }

    fn update_ssaa(&mut self, device: &Device, ssaa: Ssaa) {
        self.ssaa = ssaa;
        self.forward_size = self.ssaa.calculate_size(self.screen_size);
        self.supersampled_color_texture = Self::create_supersampled_texture(device, self.screen_size, self.ssaa);
    }

    fn update_screen_space_anti_aliasing(&mut self, device: &Device, screen_space_anti_aliasing: ScreenSpaceAntiAliasing) {
        self.screen_space_anti_aliasing = screen_space_anti_aliasing;
        self.anti_aliasing_resources = Self::create_anti_aliasing_resources(device, self.screen_space_anti_aliasing, self.screen_size);
    }

    fn update_high_quality_interface(&mut self, device: &Device, high_quality_interface: bool) {
        self.high_quality_interface = high_quality_interface;
        self.interface_size = if self.high_quality_interface {
            self.screen_size * 2.0
        } else {
            self.screen_size
        };

        self.interface_buffer_texture = Self::create_interface_texture(device, self.interface_size);
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
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            })
        })
    }

    fn light_culling_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("light culling"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R32Uint,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<TileLightIndices>() as _),
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    fn forward_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("forward"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<DirectionalLightUniforms>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::VERTEX_FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<TileLightIndices>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::CubeArray,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new((PARTITION_COUNT * size_of::<DirectionalLightPartition>()) as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<KernelUniforms>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    fn sdsm_bind_group_layout(device: &Device, msaa: Msaa) -> &'static BindGroupLayout {
        static LAYOUT_NO_MSAA: OnceLock<BindGroupLayout> = OnceLock::new();
        static LAYOUT_WITH_MSAA: OnceLock<BindGroupLayout> = OnceLock::new();

        let layout_lock = if msaa.multisampling_activated() {
            &LAYOUT_WITH_MSAA
        } else {
            &LAYOUT_NO_MSAA
        };

        layout_lock.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("sdsm"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<DirectionalLightUniforms>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: msaa.multisampling_activated(),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            })
        })
    }

    #[cfg(feature = "debug")]
    fn debug_bind_group_layout(device: &Device, msaa: Msaa) -> &'static BindGroupLayout {
        static LAYOUT_NO_MSAA: OnceLock<BindGroupLayout> = OnceLock::new();
        static LAYOUT_WITH_MSAA: OnceLock<BindGroupLayout> = OnceLock::new();

        let layout_lock = if msaa.multisampling_activated() {
            &LAYOUT_WITH_MSAA
        } else {
            &LAYOUT_NO_MSAA
        };

        layout_lock.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("debug"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<DebugUniforms>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::CubeArray,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: msaa.multisampling_activated(),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
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
        nearest_sampler: &Sampler,
        linear_sampler: &Sampler,
        texture_sampler: &Sampler,
        shadow_sampler: &Sampler,
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
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(shadow_sampler),
                },
            ],
        })
    }

    fn create_light_culling_bind_group(
        device: &Device,
        point_light_data_buffer: &Buffer<PointLightData>,
        tile_light_count_texture: &StorageTexture,
        tile_light_indices_buffer: &Buffer<TileLightIndices>,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("light culling"),
            layout: Self::light_culling_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: point_light_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(tile_light_count_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: tile_light_indices_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn create_forward_bind_group(
        device: &Device,
        directional_light_uniforms_buffer: &Buffer<DirectionalLightUniforms>,
        point_light_data_buffer: &Buffer<PointLightData>,
        tile_light_count_texture: &StorageTexture,
        tile_light_indices_buffer: &Buffer<TileLightIndices>,
        directional_shadow_map_texture: &AttachmentTexture,
        directional_shadow_translucence_texture: &AttachmentTexture,
        point_shadow_maps_texture: &CubeArrayTexture,
        directional_light_partition: &Buffer<DirectionalLightPartition>,
        kernel_uniforms_buffer: &Buffer<KernelUniforms>,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("forward"),
            layout: Self::forward_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: directional_light_uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(directional_shadow_map_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: point_light_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(tile_light_count_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: tile_light_indices_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(point_shadow_maps_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: directional_light_partition.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: kernel_uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: BindingResource::TextureView(directional_shadow_translucence_texture.get_texture_view()),
                },
            ],
        })
    }

    fn create_sdsm_bind_group(
        device: &Device,
        msaa: Msaa,
        directional_light_uniforms_buffer: &Buffer<DirectionalLightUniforms>,
        forward_depth_texture: &AttachmentTexture,
        partition_data_buffer: &Buffer<Partition>,
        interval_data_buffer: &Buffer<Interval>,
        bounds_data_buffer: &Buffer<Bounds>,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("sdsm"),
            layout: Self::sdsm_bind_group_layout(device, msaa),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: directional_light_uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(forward_depth_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: partition_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: interval_data_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: bounds_data_buffer.as_entire_binding(),
                },
            ],
        })
    }

    #[cfg(feature = "debug")]
    fn create_debug_bind_group(
        device: &Device,
        msaa: Msaa,
        debug_uniforms_buffer: &Buffer<DebugUniforms>,
        picker_buffer_texture: &AttachmentTexture,
        directional_shadow_map_texture: &AttachmentTexture,
        tile_light_count_texture: &StorageTexture,
        point_shadow_maps_texture: &CubeArrayTexture,
        forward_depth_texture: &AttachmentTexture,
        partition_data_buffer: &Buffer<Partition>,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some("debug"),
            layout: Self::debug_bind_group_layout(device, msaa),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: debug_uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(picker_buffer_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(directional_shadow_map_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(tile_light_count_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(point_shadow_maps_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(forward_depth_texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: partition_data_buffer.as_entire_binding(),
                },
            ],
        })
    }
}

fn calculate_light_tile_count(forward_size: ScreenSize) -> (u32, u32) {
    let tile_count_x = (forward_size.width as u32).div_ceil(LIGHT_TILE_SIZE);
    let tile_count_y = (forward_size.height as u32).div_ceil(LIGHT_TILE_SIZE);
    (tile_count_x, tile_count_y)
}

struct PickerTextures {
    picker_buffer_texture: AttachmentTexture,
    picker_depth_texture: AttachmentTexture,
}

struct ForwardTextures {
    forward_color_texture: AttachmentTexture,
    forward_depth_texture: AttachmentTexture,
    forward_accumulation_texture: AttachmentTexture,
    forward_revealage_texture: AttachmentTexture,
    tile_light_count_texture: StorageTexture,
}

pub(crate) enum AntiAliasingResources {
    None,
    Fxaa(Box<FxaaResources>),
}

pub(crate) struct FxaaResources {
    color_with_luma_texture: AttachmentTexture,
}
