use korangar_interface::MouseMode;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::event::{ClickHandler, DropHandler, Event, EventQueue};
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolver, WindowLayout};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use ragnarok_packets::SkillLevel;
use rust_state::{Context, Path};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::SkillSource;
use crate::inventory::{LearnableSkill, LearnedSkill};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::ClientState;

struct LevelDisplay {
    level: Option<SkillLevel>,
    maximum_level: SkillLevel,
    string: Option<String>,
}

impl Default for LevelDisplay {
    fn default() -> Self {
        Self {
            level: None,
            maximum_level: SkillLevel(0),
            string: Default::default(),
        }
    }
}

impl LevelDisplay {
    fn update(&mut self, new_level: Option<SkillLevel>, new_maximum_level: SkillLevel) {
        if self.string.is_none() || new_level != self.level || new_maximum_level != self.maximum_level {
            self.string = match new_level {
                Some(level) => Some(format!("{}/{}", level.0, new_maximum_level.0)),
                None => Some(new_maximum_level.0.to_string()),
            };
            self.level = new_level;
            self.maximum_level = new_maximum_level;
        }
    }
}

struct SkillBoxHandler<A> {
    skill_path: A,
    source: SkillSource,
}

impl<A> SkillBoxHandler<A> {
    fn new(skill_path: A, source: SkillSource) -> Self {
        Self { skill_path, source }
    }
}

impl<A> ClickHandler<ClientState> for SkillBoxHandler<A>
where
    A: Path<ClientState, LearnableSkill, false>,
{
    fn handle_click(&self, state: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
        // SAFETY:
        //
        // Unwrapping here is fine since we only register the handler if the slot has a
        // skill.
        let skill = state.try_get(&self.skill_path).unwrap().clone();

        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Custom {
                mode: MouseInputMode::MoveSkill {
                    skill,
                    source: self.source,
                },
            },
        });
    }
}

impl<P> DropHandler<ClientState> for SkillBoxHandler<P>
where
    P: Path<ClientState, LearnableSkill, false>,
{
    fn handle_drop(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        if let MouseMode::Custom {
            mode: MouseInputMode::MoveSkill { source, skill },
        } = mouse_mode
        {
            queue.queue(InputEvent::MoveSkill {
                source: *source,
                destination: self.source,
                skill: skill.clone(),
            });
        }
    }
}

pub struct SkillBox<A, B> {
    learnable_skill_path: A,
    learned_skill_path: B,
    handler: SkillBoxHandler<A>,
    level_display: LevelDisplay,
}

impl<A, B> SkillBox<A, B>
where
    A: Copy,
    B: Copy,
{
    /// This function is supposed to be called from a component macro
    /// and not intended to be called manually.
    #[inline(always)]
    pub fn component_new(learnable_skill_path: A, learned_skill_path: B, source: SkillSource) -> Self {
        Self {
            learnable_skill_path,
            learned_skill_path,
            handler: SkillBoxHandler::new(learnable_skill_path, source),
            level_display: LevelDisplay::default(),
        }
    }
}

impl<A, B> Element<ClientState> for SkillBox<A, B>
where
    A: Path<ClientState, LearnableSkill, false>,
    B: Path<ClientState, LearnedSkill, false>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &Context<ClientState>,
        _: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, ClientState>,
    ) -> Self::LayoutInfo {
        let area = resolver.with_height(40.0);

        if let Some(learnable_skill) = state.try_get(&self.learnable_skill_path) {
            let skill_level = state.try_get(&self.learned_skill_path).map(|skill| skill.skill_level);
            self.level_display.update(skill_level, learnable_skill.maximum_level);
        }

        Self::LayoutInfo { area }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let (is_hovered, background_color) = match layout.get_mouse_mode() {
            MouseMode::Custom {
                mode: MouseInputMode::MoveSkill { .. },
            } => match layout_info.area.check().any_mouse_mode().run(layout) {
                true => {
                    // Since we are not in default mouse mode we need to mark the window as
                    // hovered.
                    layout.set_hovered();

                    (true, Color::rgb_u8(80, 180, 180))
                }
                false => (false, Color::rgb_u8(180, 180, 80)),
            },
            _ => match layout_info.area.check().run(layout) {
                true => (true, Color::rgb_u8(60, 60, 60)),
                false => (false, Color::rgb_u8(40, 40, 40)),
            },
        };

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(20.0),
            background_color,
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if is_hovered {
            layout.register_drop_handler(&self.handler);
        }

        if let Some(skill) = state.try_get(&self.learnable_skill_path) {
            let color = match state.try_get(&self.learned_skill_path).is_some() {
                true => Color::WHITE,
                false => Color::monochrome_u8(80),
            };

            layout.add_sprite(layout_info.area, &skill.actions, &skill.sprite, &skill.animation_state, color);

            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &self.handler);

                struct SkillBoxTooltip;
                layout.add_tooltip(&skill.skill_name, SkillBoxTooltip.tooltip_id());
            }

            layout.add_text(
                layout_info.area,
                self.level_display.string.as_ref().unwrap(),
                // TODO: Put this in the theme
                FontSize(12.0),
                // TODO: Put this in the theme
                Color::rgb_u8(255, 200, 255),
                // TODO: Put this in the theme
                Color::rgb_u8(255, 160, 60),
                // TODO: Put this in the theme
                HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                // TODO: Put this in the theme
                VerticalAlignment::Bottom { offset: 3.0 },
                OverflowBehavior::Shrink,
            );
        }
    }
}
