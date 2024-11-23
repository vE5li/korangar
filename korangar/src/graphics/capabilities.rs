use std::num::NonZeroU32;

#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use wgpu::{Adapter, Features, Limits, TextureFormat, TextureFormatFeatureFlags};

use crate::graphics::{Msaa, RENDER_TO_TEXTURE_DEPTH_FORMAT, RENDER_TO_TEXTURE_FORMAT};

const MAX_TEXTURES_PER_SHADER_STAGE: u32 = 1024;
const MAX_TEXTURE_SIZE: u32 = 8192;

pub struct Capabilities {
    supported_msaa: Vec<Msaa>,
    bindless: bool,
    multidraw_indirect: bool,
    #[cfg(feature = "debug")]
    polygon_mode_line: bool,
    required_features: Features,
    required_limits: Limits,
}

impl Capabilities {
    pub fn from_adapter(adapter: &Adapter) -> Self {
        let adapter_features = adapter.features();
        let adapter_limits = adapter.limits();

        // We need to test all textures that we use for MSAA
        // which sample count they support.
        let supported_msaa = determine_supported_msaa(adapter, &[RENDER_TO_TEXTURE_FORMAT, RENDER_TO_TEXTURE_DEPTH_FORMAT]);

        let mut capabilities = Self {
            supported_msaa,
            bindless: false,
            multidraw_indirect: false,
            #[cfg(feature = "debug")]
            polygon_mode_line: false,
            required_features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
            required_limits: Limits::default().using_resolution(adapter.limits()),
        };

        if capabilities.required_limits.max_texture_dimension_2d < MAX_TEXTURE_SIZE {
            capabilities.required_limits.max_texture_dimension_2d = MAX_TEXTURE_SIZE;
        }

        #[cfg(feature = "debug")]
        {
            Self::check_feature(adapter_features, Features::INDIRECT_FIRST_INSTANCE);
            Self::check_feature(adapter_features, Features::MULTI_DRAW_INDIRECT);
            Self::check_feature(adapter_features, Features::PARTIALLY_BOUND_BINDING_ARRAY);
            Self::check_feature(
                adapter_features,
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            );
            Self::check_feature(adapter_features, Features::TEXTURE_BINDING_ARRAY);
            Self::check_feature(adapter_features, Features::POLYGON_MODE_LINE);
        }

        if adapter_features.contains(
            Features::PARTIALLY_BOUND_BINDING_ARRAY
                | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                | Features::TEXTURE_BINDING_ARRAY,
        ) && adapter_limits.max_sampled_textures_per_shader_stage >= MAX_TEXTURES_PER_SHADER_STAGE
        {
            capabilities.bindless = true;
            capabilities.required_features |= Features::PARTIALLY_BOUND_BINDING_ARRAY
                | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING
                | Features::TEXTURE_BINDING_ARRAY;
            capabilities.required_limits.max_sampled_textures_per_shader_stage = MAX_TEXTURES_PER_SHADER_STAGE;
        }

        if adapter_features.contains(Features::INDIRECT_FIRST_INSTANCE | Features::MULTI_DRAW_INDIRECT) {
            capabilities.multidraw_indirect = true;
            capabilities.required_features |= Features::INDIRECT_FIRST_INSTANCE | Features::MULTI_DRAW_INDIRECT;
        }

        #[cfg(feature = "debug")]
        if adapter_features.contains(Features::POLYGON_MODE_LINE) {
            capabilities.polygon_mode_line = true;
            capabilities.required_features |= Features::POLYGON_MODE_LINE;
        }

        capabilities
    }

    pub fn get_supported_msaa(&self) -> &[Msaa] {
        self.supported_msaa.as_ref()
    }

    pub fn get_required_features(&self) -> Features {
        self.required_features
    }

    pub fn get_required_limits(&self) -> Limits {
        self.required_limits.clone()
    }

    /// Returns the maximum size of 2D textures.
    pub fn get_max_texture_dimension_2d(&self) -> u32 {
        self.required_limits.max_texture_dimension_2d
    }

    /// Returns the maximum count of textures inside a binding array.
    pub fn get_max_texture_binding_array_count(&self) -> Option<NonZeroU32> {
        // We need room for 8 textures for the screen bind group.
        NonZeroU32::new(self.required_limits.max_sampled_textures_per_shader_stage.saturating_sub(8))
    }

    /// Returns `true` if the backend supports all features needed for multidraw
    /// indirect.
    pub fn supports_multidraw_indirect(&self) -> bool {
        self.multidraw_indirect
    }

    /// Returns `true` if the backend supports all features and limits to
    /// support bindless fully.
    pub fn supports_bindless(&self) -> bool {
        self.bindless
    }

    /// Returns `true` if the backend allows drawing triangles as lines
    /// (wireframe) instead of filled.
    #[cfg(feature = "debug")]
    pub fn supports_polygon_mode_line(&self) -> bool {
        self.polygon_mode_line
    }

    #[cfg(feature = "debug")]
    fn check_feature(features: Features, feature: Features) {
        let supported = match features.contains(feature) {
            true => "supported".green(),
            false => "unsupported".yellow(),
        };
        print_debug!("{:?}: {}", feature, supported);
    }
}

fn determine_supported_msaa(adapter: &Adapter, texture_formats: &[TextureFormat]) -> Vec<Msaa> {
    let mut supported_msaa = vec![Msaa::Off];

    let msaa_levels = [
        (TextureFormatFeatureFlags::MULTISAMPLE_X2, Msaa::X2),
        (TextureFormatFeatureFlags::MULTISAMPLE_X4, Msaa::X4),
        (TextureFormatFeatureFlags::MULTISAMPLE_X8, Msaa::X8),
        (TextureFormatFeatureFlags::MULTISAMPLE_X16, Msaa::X16),
    ];

    for (flag, level) in msaa_levels.into_iter() {
        if texture_formats.iter().all(|&format| {
            let features = adapter.get_texture_format_features(format);
            features.flags.contains(flag)
        }) {
            supported_msaa.push(level);
        }
    }

    supported_msaa
}
