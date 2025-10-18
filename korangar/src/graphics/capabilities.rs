#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use wgpu::{Adapter, Features, Limits, TextureFormat, TextureFormatFeatureFlags};

use crate::graphics::{Msaa, RENDER_TO_TEXTURE_DEPTH_FORMAT, RENDER_TO_TEXTURE_FORMAT};

pub const MAX_BINDING_ARRAY_ELEMENTS_PER_SHADER_STAGE: u32 = 10240;

/// The maximum texture size that is guaranteed by the graphic engine to be
/// available.
pub const MAX_TEXTURE_SIZE: u32 = 8192;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BindlessSupport {
    /// Full bindless support.
    Full,
    /// Limited bindless support (missing PARTIALLY_BOUND_BINDING_ARRAY).
    Limited,
    /// Bindless is not supported at all.
    None,
}

pub struct Capabilities {
    supported_msaa: Vec<Msaa>,
    bindless: BindlessSupport,
    multidraw_indirect: bool,
    clamp_to_border: bool,
    texture_compression: bool,
    #[cfg(feature = "debug")]
    polygon_mode_line: bool,
    required_features: Features,
    required_limits: Limits,
}

impl Capabilities {
    pub fn from_adapter(adapter: &Adapter) -> Self {
        let adapter_features = adapter.features();
        let adapter_limits = adapter.limits();

        // We need to test all textures that we use for MSAA which sample count they
        // support.
        let supported_msaa = determine_supported_msaa(adapter, &[RENDER_TO_TEXTURE_FORMAT, RENDER_TO_TEXTURE_DEPTH_FORMAT]);

        let required_limits = Limits::default().using_resolution(adapter.limits());

        // WebWGPUs default limit is 8192, which we assert here, because our font maps
        // are designed to work inside this limit.
        assert!(required_limits.max_texture_dimension_2d >= MAX_TEXTURE_SIZE);

        let mut capabilities = Self {
            supported_msaa,
            bindless: BindlessSupport::None,
            multidraw_indirect: false,
            clamp_to_border: false,
            texture_compression: false,
            #[cfg(feature = "debug")]
            polygon_mode_line: false,
            required_features: Features::empty(),
            required_limits,
        };

        #[cfg(feature = "debug")]
        {
            Self::check_limit(
                "max_binding_array_elements_per_shader_stage",
                adapter_limits.max_binding_array_elements_per_shader_stage,
                MAX_BINDING_ARRAY_ELEMENTS_PER_SHADER_STAGE,
            );
            Self::check_feature(adapter_features, Features::ADDRESS_MODE_CLAMP_TO_BORDER);
            Self::check_feature(adapter_features, Features::ADDRESS_MODE_CLAMP_TO_ZERO);
            Self::check_feature(adapter_features, Features::INDIRECT_FIRST_INSTANCE);
            Self::check_feature(adapter_features, Features::PARTIALLY_BOUND_BINDING_ARRAY);
            Self::check_feature(
                adapter_features,
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
            );
            Self::check_feature(adapter_features, Features::TEXTURE_COMPRESSION_BC);
            Self::check_feature(adapter_features, Features::TEXTURE_BINDING_ARRAY);
            Self::check_feature(adapter_features, Features::POLYGON_MODE_LINE);
        }

        if adapter_features
            .contains(Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING | Features::TEXTURE_BINDING_ARRAY)
            && adapter_limits.max_binding_array_elements_per_shader_stage >= MAX_BINDING_ARRAY_ELEMENTS_PER_SHADER_STAGE
        {
            capabilities.bindless = BindlessSupport::Limited;

            capabilities.required_features |=
                Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING | Features::TEXTURE_BINDING_ARRAY;

            capabilities.required_limits.max_binding_array_elements_per_shader_stage = MAX_BINDING_ARRAY_ELEMENTS_PER_SHADER_STAGE;

            if adapter_features.contains(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
                capabilities.bindless = BindlessSupport::Full;
                capabilities.required_features |= Features::PARTIALLY_BOUND_BINDING_ARRAY;
            }
        }

        if adapter_features.contains(Features::INDIRECT_FIRST_INSTANCE) {
            capabilities.multidraw_indirect = true;
            capabilities.required_features |= Features::INDIRECT_FIRST_INSTANCE;
        }

        if adapter_features.contains(Features::ADDRESS_MODE_CLAMP_TO_BORDER | Features::ADDRESS_MODE_CLAMP_TO_ZERO) {
            capabilities.clamp_to_border = true;
            capabilities.required_features |= Features::ADDRESS_MODE_CLAMP_TO_BORDER | Features::ADDRESS_MODE_CLAMP_TO_ZERO;
        }

        if adapter_features.contains(Features::TEXTURE_COMPRESSION_BC) {
            capabilities.texture_compression = true;
            capabilities.required_features |= Features::TEXTURE_COMPRESSION_BC;
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
    pub const fn get_max_texture_binding_array_count(&self) -> u32 {
        MAX_BINDING_ARRAY_ELEMENTS_PER_SHADER_STAGE
    }

    /// Returns `true` if the backend supports all features needed for multidraw
    /// indirect.
    pub fn supports_multidraw_indirect(&self) -> bool {
        self.multidraw_indirect
    }

    /// Returns the supported bindless level.
    pub fn bindless_support(&self) -> BindlessSupport {
        self.bindless
    }

    /// Returns `true` if the backend allows clamping the border of a texture to
    /// a specific value.
    pub fn supports_clamp_to_border(&self) -> bool {
        self.clamp_to_border
    }

    /// Returns `true` if the backend supports BC texture compression.
    pub fn supports_texture_compression(&self) -> bool {
        self.texture_compression
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
        print_debug!("Feature {}: {}", feature, supported);
    }

    #[cfg(feature = "debug")]
    fn check_limit(name: &str, actual: u32, required: u32) {
        let status = match actual < required {
            true => format!("{} ({} < {})", "warn".yellow(), actual, required),
            false => format!("{} ({})", "ok".green(), actual),
        };
        print_debug!("Limit {}: {}", name, status);
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
