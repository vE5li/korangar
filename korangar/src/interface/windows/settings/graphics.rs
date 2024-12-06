use korangar_interface::elements::{ElementWrap, PickList, PrototypeElement, StateButtonBuilder, Text};
use korangar_interface::state::{TrackedState, TrackedStateBinary};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};

use crate::graphics::{LimitFramerate, Msaa, PresentModeInfo, ScreenSpaceAntiAliasing, ShadowDetail, Ssaa, TextureSamplerType};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::settings::LightningMode;

pub struct GraphicsSettingsWindow<
    LightningRenderMode,
    Vsync,
    FramerateLimit,
    TripleBuffering,
    TextureFiltering,
    Multisampling,
    Supersampling,
    ScreenAntiAliasing,
    Shadow,
    HighQualityInterface,
> where
    LightningRenderMode: TrackedState<LightningMode> + 'static,
    Vsync: TrackedStateBinary<bool>,
    FramerateLimit: TrackedState<LimitFramerate> + 'static,
    TripleBuffering: TrackedStateBinary<bool>,
    TextureFiltering: TrackedState<TextureSamplerType> + 'static,
    Multisampling: TrackedState<Msaa> + 'static,
    Supersampling: TrackedState<Ssaa> + 'static,
    ScreenAntiAliasing: TrackedState<ScreenSpaceAntiAliasing> + 'static,
    Shadow: TrackedState<ShadowDetail> + 'static,
    HighQualityInterface: TrackedStateBinary<bool>,
{
    present_mode_info: PresentModeInfo,
    supported_msaa: Vec<(String, Msaa)>,
    lightning_mode: LightningRenderMode,
    vsync: Vsync,
    limit_framerate: FramerateLimit,
    triple_buffering: TripleBuffering,
    texture_filtering: TextureFiltering,
    msaa: Multisampling,
    ssaa: Supersampling,
    screen_space_anti_aliasing: ScreenAntiAliasing,
    shadow_detail: Shadow,
    high_quality_interface: HighQualityInterface,
}

impl<
        LightningRenderMode,
        Vsync,
        FramerateLimit,
        TripleBuffering,
        TextureFiltering,
        Multisampling,
        Supersampling,
        ScreenAntiAliasing,
        Shadow,
        HighQualityInterface,
    >
    GraphicsSettingsWindow<
        LightningRenderMode,
        Vsync,
        FramerateLimit,
        TripleBuffering,
        TextureFiltering,
        Multisampling,
        Supersampling,
        ScreenAntiAliasing,
        Shadow,
        HighQualityInterface,
    >
where
    LightningRenderMode: TrackedState<LightningMode> + 'static,
    Vsync: TrackedStateBinary<bool>,
    FramerateLimit: TrackedState<LimitFramerate> + 'static,
    TripleBuffering: TrackedStateBinary<bool>,
    TextureFiltering: TrackedState<TextureSamplerType> + 'static,
    Multisampling: TrackedState<Msaa> + 'static,
    Supersampling: TrackedState<Ssaa> + 'static,
    ScreenAntiAliasing: TrackedState<ScreenSpaceAntiAliasing> + 'static,
    Shadow: TrackedState<ShadowDetail> + 'static,
    HighQualityInterface: TrackedStateBinary<bool>,
{
    pub const WINDOW_CLASS: &'static str = "graphics_settings";

    pub fn new(
        present_mode_info: PresentModeInfo,
        supported_msaa: Vec<(String, Msaa)>,
        lightning_mode: LightningRenderMode,
        vsync: Vsync,
        limit_framerate: FramerateLimit,
        triple_buffering: TripleBuffering,
        texture_filtering: TextureFiltering,
        msaa: Multisampling,
        ssaa: Supersampling,
        screen_space_anti_aliasing: ScreenAntiAliasing,
        shadow_detail: Shadow,
        high_quality_interface: HighQualityInterface,
    ) -> Self {
        Self {
            present_mode_info,
            supported_msaa,
            lightning_mode,
            vsync,
            limit_framerate,
            triple_buffering,
            texture_filtering,
            msaa,
            ssaa,
            screen_space_anti_aliasing,
            shadow_detail,
            high_quality_interface,
        }
    }
}

impl<
        LightningRenderMode,
        Vsync,
        FramerateLimit,
        TripleBuffering,
        TextureFiltering,
        Multisampling,
        Supersampling,
        ScreenAntiAliasing,
        Shadow,
        HighQualityInterface,
    > PrototypeWindow<InterfaceSettings>
    for GraphicsSettingsWindow<
        LightningRenderMode,
        Vsync,
        FramerateLimit,
        TripleBuffering,
        TextureFiltering,
        Multisampling,
        Supersampling,
        ScreenAntiAliasing,
        Shadow,
        HighQualityInterface,
    >
where
    LightningRenderMode: TrackedState<LightningMode> + 'static,
    Vsync: TrackedStateBinary<bool>,
    FramerateLimit: TrackedState<LimitFramerate> + 'static,
    TripleBuffering: TrackedStateBinary<bool>,
    TextureFiltering: TrackedState<TextureSamplerType> + 'static,
    Multisampling: TrackedState<Msaa> + 'static,
    Supersampling: TrackedState<Ssaa> + 'static,
    ScreenAntiAliasing: TrackedState<ScreenSpaceAntiAliasing> + 'static,
    Shadow: TrackedState<ShadowDetail> + 'static,
    HighQualityInterface: TrackedStateBinary<bool>,
{
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let mut elements = vec![
            Text::default().with_text("Lightning mode").with_width(dimension_bound!(50%)).wrap(),
            PickList::default()
                .with_options(vec![("Classic", LightningMode::Classic), ("Enhanced", LightningMode::Enhanced)])
                .with_selected(self.lightning_mode.clone())
                .with_event(Box::new(Vec::new))
                .with_width(dimension_bound!(!))
                .wrap(),
            StateButtonBuilder::new()
                .with_text("Triple buffering")
                .with_event(self.triple_buffering.toggle_action())
                .with_remote(self.triple_buffering.new_remote())
                .build()
                .wrap(),
            Text::default()
                .with_text("Texture filtering")
                .with_width(dimension_bound!(50%))
                .wrap(),
            PickList::default()
                .with_options(vec![
                    ("Nearest", TextureSamplerType::Nearest),
                    ("Linear", TextureSamplerType::Linear),
                    ("Anisotropic x4", TextureSamplerType::Anisotropic(4)),
                    ("Anisotropic x8", TextureSamplerType::Anisotropic(8)),
                    ("Anisotropic x16", TextureSamplerType::Anisotropic(16)),
                ])
                .with_selected(self.texture_filtering.clone())
                .with_event(Box::new(Vec::new))
                .with_width(dimension_bound!(!))
                .wrap(),
            Text::default().with_text("Multisampling").with_width(dimension_bound!(50%)).wrap(),
            PickList::default()
                .with_options(self.supported_msaa.clone())
                .with_selected(self.msaa.clone())
                .with_event(Box::new(Vec::new))
                .with_width(dimension_bound!(!))
                .wrap(),
            Text::default().with_text("Supersampling").with_width(dimension_bound!(50%)).wrap(),
            PickList::default()
                .with_options(vec![("Off", Ssaa::Off), ("x2", Ssaa::X2), ("x3", Ssaa::X3), ("x4", Ssaa::X4)])
                .with_selected(self.ssaa.clone())
                .with_event(Box::new(Vec::new))
                .with_width(dimension_bound!(!))
                .wrap(),
            Text::default()
                .with_text("Screen space AA")
                .with_width(dimension_bound!(50%))
                .wrap(),
            PickList::default()
                .with_options(vec![
                    ("Off", ScreenSpaceAntiAliasing::Off),
                    ("FXAA", ScreenSpaceAntiAliasing::Fxaa),
                    ("CMAA2", ScreenSpaceAntiAliasing::Cmaa2),
                ])
                .with_selected(self.screen_space_anti_aliasing.clone())
                .with_event(Box::new(Vec::new))
                .with_width(dimension_bound!(!))
                .wrap(),
            Text::default().with_text("Shadow detail").with_width(dimension_bound!(50%)).wrap(),
            PickList::default()
                .with_options(vec![
                    ("Low", ShadowDetail::Low),
                    ("Medium", ShadowDetail::Medium),
                    ("High", ShadowDetail::High),
                    ("Ultra", ShadowDetail::Ultra),
                ])
                .with_selected(self.shadow_detail.clone())
                .with_event(Box::new(Vec::new))
                .with_width(dimension_bound!(!))
                .wrap(),
            StateButtonBuilder::new()
                .with_text("High Quality Interface")
                .with_event(self.high_quality_interface.toggle_action())
                .with_remote(self.high_quality_interface.new_remote())
                .build()
                .wrap(),
            application.to_element("Interface settings".to_string()),
        ];

        // TODO: Instead of not showing these options, disable the checkboxes and add a
        //       tooltip
        if self.present_mode_info.supports_immediate || self.present_mode_info.supports_mailbox {
            elements.insert(
                2,
                StateButtonBuilder::new()
                    .with_text("Enable VSYNC")
                    .with_event(self.vsync.toggle_action())
                    .with_remote(self.vsync.new_remote())
                    .build()
                    .wrap(),
            );
            elements.insert(
                3,
                Text::default()
                    .with_text("Limit framerate")
                    .with_width(dimension_bound!(50%))
                    .wrap(),
            );
            elements.insert(
                2,
                PickList::default()
                    .with_options(vec![
                        ("Unlimited", LimitFramerate::Unlimited),
                        ("30 Hz", LimitFramerate::Limit(30)),
                        ("60 Hz", LimitFramerate::Limit(60)),
                        ("120 Hz", LimitFramerate::Limit(120)),
                        ("144 Hz", LimitFramerate::Limit(144)),
                        ("240 Hz", LimitFramerate::Limit(240)),
                    ])
                    .with_selected(self.limit_framerate.clone())
                    .with_event(Box::new(Vec::new))
                    .with_width(dimension_bound!(!))
                    .wrap(),
            );
        }

        WindowBuilder::new()
            .with_title("Graphics Settings".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
