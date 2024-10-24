use derive_new::new;
use korangar_interface::application::FontSizeTrait;
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::{ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;

use crate::graphics::Color;
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition};
use crate::interface::resource::{Move, PartialMove, SkillSource};
use crate::interface::theme::InterfaceTheme;
use crate::inventory::Skill;
use crate::loaders::FontSize;
use crate::renderer::InterfaceRenderer;

#[derive(new)]
pub struct SkillBox {
    skill: Option<Skill>,
    source: SkillSource,
    highlight: Box<dyn Fn(&MouseInputMode) -> bool>,
    #[new(default)]
    state: ElementState<InterfaceSettings>,
}

impl Element<InterfaceSettings> for SkillBox {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        self.skill.is_some()
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        _application: &InterfaceSettings,
        _theme: &InterfaceTheme,
    ) {
        self.state.resolve(placement_resolver, &size_bound!(30, 30));
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match self.skill.is_some() || matches!(mouse_mode, MouseInputMode::MoveSkill(..)) {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<InterfaceSettings>> {
        if let Some(skill) = &self.skill {
            return vec![ClickAction::Move(PartialMove::Skill {
                source: self.source,
                skill: skill.clone(),
            })];
        }

        Vec::new()
    }

    fn drop_resource(&mut self, drop_resource: PartialMove) -> Option<Move> {
        let PartialMove::Skill { source, skill } = drop_resource else {
            return None;
        };

        (source != self.source).then_some(Move::Skill {
            source,
            destination: self.source,
            skill,
        })
    }

    fn render(
        &self,
        renderer: &InterfaceRenderer,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element<InterfaceSettings>>,
        focused_element: Option<&dyn Element<InterfaceSettings>>,
        mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self.state.element_renderer(renderer, application, parent_position, screen_clip);

        let highlight = (self.highlight)(mouse_mode);
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true if highlight => Color::rgba_u8(60, 160, 160, 255),
            true if matches!(mouse_mode, MouseInputMode::None) => theme.button.hovered_background_color.get(),
            false if highlight => Color::rgba_u8(160, 160, 60, 255),
            _ => theme.button.background_color.get(),
        };

        renderer.render_background(CornerRadius::uniform(5.0), background_color);

        if let Some(skill) = &self.skill {
            skill.actions.render(
                renderer.renderer,
                &skill.sprite,
                &skill.animation_state,
                renderer.position + ScreenPosition::uniform(15.0 * application.get_scaling_factor()),
                0,
                Color::WHITE,
                application,
            );

            renderer.render_text(
                &format!("{}", skill.skill_level.0),
                ScreenPosition::uniform(1.0),
                Color::BLACK,
                FontSize::new(15.0),
            );
            renderer.render_text(
                &format!("{}", skill.skill_level.0),
                ScreenPosition::default(),
                Color::WHITE,
                FontSize::new(15.0),
            );
        }
    }
}
