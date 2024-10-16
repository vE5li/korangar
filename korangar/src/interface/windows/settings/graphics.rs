use korangar_interface::element::PrototypeElement;
use korangar_interface::event::Toggle;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, Path};

use crate::GraphicsSettings;
use crate::graphics::{
    LimitFramerate, Msaa, PresentModeInfo, ScreenSpaceAntiAliasing, ShadowDetail, ShadowQuality, Ssaa, TextureSamplerType,
};
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::settings::{GraphicsSettingsPathExt, LightingMode};
use crate::state::{ClientState, ClientThemeType};

pub struct GraphicsSettingsWindow<P> {
    path: P,
}

impl<P> GraphicsSettingsWindow<P> {
    pub const WINDOW_CLASS: &'static str = "graphics_settings";

    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for GraphicsSettingsWindow<P>
where
    P: Path<ClientState, GraphicsSettings>,
{
    fn window_class() -> Option<&'static str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // let mut elements = vec![
        //     Text::default().with_text("Lighting
        // mode").with_width(dimension_bound!(50%)).wrap(),
        //     PickList::default()
        //         .with_options(vec![("Classic", LightingMode::Classic), ("Enhanced",
        // LightingMode::Enhanced)])         .with_selected(self.lighting_mode.
        // clone())         .with_event(Box::new(Vec::new))
        //         .with_width(dimension_bound!(!))
        //         .wrap(),
        //     StateButtonBuilder::new()
        //         .with_text("Triple buffering")
        //         .with_event(self.triple_buffering.toggle_action())
        //         .with_remote(self.triple_buffering.new_remote())
        //         .build()
        //         .wrap(),
        //     Text::default()
        //         .with_text("Texture filtering")
        //         .with_width(dimension_bound!(50%))
        //         .wrap(),
        //     PickList::default()
        //         .with_options(vec![
        //             ("Nearest", TextureSamplerType::Nearest),
        //             ("Linear", TextureSamplerType::Linear),
        //             ("Anisotropic x4", TextureSamplerType::Anisotropic(4)),
        //             ("Anisotropic x8", TextureSamplerType::Anisotropic(8)),
        //             ("Anisotropic x16", TextureSamplerType::Anisotropic(16)),
        //         ])
        //         .with_selected(self.texture_filtering.clone())
        //         .with_event(Box::new(Vec::new))
        //         .with_width(dimension_bound!(!))
        //         .wrap(),
        //     Text::default().with_text("Multisampling").with_width(dimension_bound!
        // (50%)).wrap(),     PickList::default()
        //         .with_options(self.supported_msaa.clone())
        //         .with_selected(self.msaa.clone())
        //         .with_event(Box::new(Vec::new))
        //         .with_width(dimension_bound!(!))
        //         .wrap(),
        //     Text::default().with_text("Supersampling").with_width(dimension_bound!
        // (50%)).wrap(),     PickList::default()
        //         .with_options(vec![("Off", Ssaa::Off), ("x2", Ssaa::X2), ("x3",
        // Ssaa::X3), ("x4", Ssaa::X4)])         .with_selected(self.ssaa.
        // clone())         .with_event(Box::new(Vec::new))
        //         .with_width(dimension_bound!(!))
        //         .wrap(),
        //     Text::default()
        //         .with_text("Screen space AA")
        //         .with_width(dimension_bound!(50%))
        //         .wrap(),
        //     PickList::default()
        //         .with_options(vec![
        //             ("Off", ScreenSpaceAntiAliasing::Off),
        //             ("FXAA", ScreenSpaceAntiAliasing::Fxaa),
        //         ])
        //         .with_selected(self.screen_space_anti_aliasing.clone())
        //         .with_event(Box::new(Vec::new))
        //         .with_width(dimension_bound!(!))
        //         .wrap(),
        //     Text::default().with_text("Shadow
        // quality").with_width(dimension_bound!(50%)).wrap(),
        //     PickList::default()
        //         .with_options(vec![
        //             ("Hard", ShadowQuality::Hard),
        //             ("Soft (PCF)", ShadowQuality::SoftPCF),
        //             ("Soft (PCSS x8)", ShadowQuality::SoftPCSSx8),
        //             ("Soft (PCSS x16)", ShadowQuality::SoftPCSSx16),
        //             ("Soft (PCSS x32)", ShadowQuality::SoftPCSSx32),
        //             ("Soft (PCSS x64)", ShadowQuality::SoftPCSSx64),
        //         ])
        //         .with_selected(self.shadow_quality.clone())
        //         .with_event(Box::new(Vec::new))
        //         .with_width(dimension_bound!(!))
        //         .wrap(),
        //     Text::default().with_text("Shadow
        // detail").with_width(dimension_bound!(50%)).wrap(),
        //     PickList::default()
        //         .with_options(vec![
        //             ("Normal", ShadowDetail::Normal),
        //             ("Ultra", ShadowDetail::Ultra),
        //             ("Insane", ShadowDetail::Insane),
        //         ])
        //         .with_selected(self.shadow_detail.clone())
        //         .with_event(Box::new(Vec::new))
        //         .with_width(dimension_bound!(!))
        //         .wrap(),
        //     StateButtonBuilder::new()
        //         .with_text("High Quality Interface")
        //         .with_event(self.high_quality_interface.toggle_action())
        //         .with_remote(self.high_quality_interface.new_remote())
        //         .build()
        //         .wrap(),
        //     application.to_element("Interface settings".to_string()),
        // ];
        //
        // // TODO: Instead of not showing these options, disable the checkboxes and add
        // a //       tooltip
        // if self.present_mode_info.supports_immediate ||
        // self.present_mode_info.supports_mailbox {     elements.insert(
        //         2,
        //         StateButtonBuilder::new()
        //             .with_text("Enable VSYNC")
        //             .with_event(self.vsync.toggle_action())
        //             .with_remote(self.vsync.new_remote())
        //             .build()
        //             .wrap(),
        //     );
        //     elements.insert(
        //         3,
        //         Text::default()
        //             .with_text("Limit framerate")
        //             .with_width(dimension_bound!(50%))
        //             .wrap(),
        //     );
        //     elements.insert(
        //         4,
        //         PickList::default()
        //             .with_options(vec![
        //                 ("Unlimited", LimitFramerate::Unlimited),
        //                 ("30 Hz", LimitFramerate::Limit(30)),
        //                 ("60 Hz", LimitFramerate::Limit(60)),
        //                 ("120 Hz", LimitFramerate::Limit(120)),
        //                 ("144 Hz", LimitFramerate::Limit(144)),
        //                 ("240 Hz", LimitFramerate::Limit(240)),
        //             ])
        //             .with_selected(self.limit_framerate.clone())
        //             .with_event(Box::new(Vec::new))
        //             .with_width(dimension_bound!(!))
        //             .wrap(),
        //     );
        // }
        //
        // WindowBuilder::new()
        //     .with_title("".to_string())
        //     .with_class(Self::WINDOW_CLASS.to_string())
        //     .with_size_bound(size_bound!(200 > 300 < 400, ?))
        //     .with_elements(elements)
        //     .closable()
        //     .build(window_cache, application, available_space)

        let elements = (state_button! {
            text: "TripleBuffering",
            state: self.path.triple_buffering(),
            event: Toggle(self.path.triple_buffering()),
        },);

        window! {
            title: "Graphics Settings",
            theme: ClientThemeType::Menu,
            window_id: 0,
            elements: elements,
        }
    }
}
