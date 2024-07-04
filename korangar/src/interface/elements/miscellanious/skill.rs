use derive_new::new;
use korangar_interface::application::{FontSizeTrait, ScalingTrait};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::{ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::theme::ButtonTheme;
use rust_state::{Context, Tracker};

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition};
use crate::interface::resource::{Move, PartialMove, SkillSource};
use crate::interface::theme::InterfaceTheme;
use crate::inventory::Skill;
use crate::loaders::FontSize;
use crate::{GameState, GameStateFocusedElementPath, GameStateHoveredElementPath, GameStateMouseModePath, GameStateScalePath};

#[derive(new)]
pub struct SkillBox {
    skill: Option<Skill>,
    source: SkillSource,
    highlight: Box<dyn Fn(&MouseInputMode) -> bool>,
    #[new(default)]
    state: ElementState<GameState>,
}

impl Element<GameState> for SkillBox {
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        self.skill.is_some()
    }

    fn resolve(
        &mut self,
        _state: &Tracker<GameState>,
        _theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        self.state.resolve(placement_resolver, &size_bound!(30, 30));
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<GameState> {
        match self.skill.is_some() || matches!(mouse_mode, MouseInputMode::MoveSkill(..)) {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _state: &Context<GameState>, _force_update: &mut bool) -> Vec<ClickAction<GameState>> {
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
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        let mouse_mode = state.get_safe(&GameStateMouseModePath::default());
        let hovered_element = state.get_safe(&GameStateHoveredElementPath::default());
        let focused_element = state.get_safe(&GameStateFocusedElementPath::default());
        let highlighted = self.is_cell_self(&hovered_element) || self.is_cell_self(&focused_element);

        let highlight = (self.highlight)(mouse_mode);
        let background_color = match highlighted {
            true if highlight => Color::rgba_u8(60, 160, 160, 255),
            true if matches!(mouse_mode, MouseInputMode::None) => *state.get_safe(&ButtonTheme::hovered_background_color(theme_selector)),
            false if highlight => Color::rgba_u8(160, 160, 60, 255),
            _ => *state.get_safe(&ButtonTheme::background_color(theme_selector)),
        };

        renderer.render_background(CornerRadius::uniform(5.0), background_color);

        if let Some(skill) = &self.skill {
            skill.actions.render2(
                renderer.render_target,
                renderer.renderer,
                &skill.sprite,
                &skill.animation_state,
                renderer.position + ScreenPosition::uniform(15.0 * state.get_safe(&GameStateScalePath::default()).get_factor()),
                0,
                Color::monochrome_u8(255),
                *state.get_safe(&GameStateScalePath::default()),
            );

            renderer.render_text(
                &format!("{}", skill.skill_level.0),
                ScreenPosition::uniform(1.0),
                Color::monochrome_u8(0),
                FontSize::new(15.0),
            );
            renderer.render_text(
                &format!("{}", skill.skill_level.0),
                ScreenPosition::default(),
                Color::monochrome_u8(255),
                FontSize::new(15.0),
            );
        }
    }
}
