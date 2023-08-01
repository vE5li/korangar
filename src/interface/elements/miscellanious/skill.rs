use cgmath::{Array, Vector4, Zero};
use derive_new::new;
use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;
use crate::inventory::Skill;

#[derive(new)]
pub struct SkillBox {
    skill: Option<Skill>,
    source: SkillSource,
    highlight: Box<dyn Fn(&MouseInputMode) -> bool>,
    #[new(default)]
    state: ElementState,
}

impl Element for SkillBox {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        self.skill.is_some()
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &Theme) {
        self.state.resolve(placement_resolver, &constraint!(30, 30));
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match self.skill.is_some() || matches!(mouse_mode, MouseInputMode::MoveSkill(..)) {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Option<ClickAction> {
        if let Some(skill) = &self.skill {
            return Some(ClickAction::MoveSkill(self.source, skill.clone()));
        }

        None
    }

    fn drop_skill(&mut self, skill_source: SkillSource, skill: Skill) -> Option<SkillMove> {
        (skill_source != self.source).then_some(SkillMove {
            source: skill_source,
            destination: self.source,
            skill,
        })
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let highlight = (self.highlight)(mouse_mode);
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true if highlight => Color::rgba(60, 160, 160, 255),
            true if matches!(mouse_mode, MouseInputMode::None) => *theme.button.hovered_background_color,
            false if highlight => Color::rgba(160, 160, 60, 255),
            _ => *theme.button.background_color,
        };

        renderer.render_background(Vector4::from_value(5.0), background_color);

        if let Some(skill) = &self.skill {
            skill.actions.render2(
                renderer.render_target,
                renderer.renderer,
                &skill.sprite,
                &skill.animation_state,
                renderer.position + Vector2::from_value(15.0 * *interface_settings.scaling),
                0,
                Color::monochrome(255),
                interface_settings,
            );

            renderer.render_text(
                &format!("{}", skill.skill_level.0),
                Vector2::from_value(1.0),
                Color::monochrome(0),
                15.0,
            );
            renderer.render_text(
                &format!("{}", skill.skill_level.0),
                Vector2::zero(),
                Color::monochrome(255),
                15.0,
            );
        }
    }
}
