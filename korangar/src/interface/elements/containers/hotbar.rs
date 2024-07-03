use korangar_interface::elements::{ContainerState, Element, ElementCell, ElementState, ElementWrap, Focus, WeakElementCell};
use korangar_interface::event::{ChangeEvent, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, Remote};
use ragnarok_packets::HotbarSlot;
use rust_state::Tracker;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::elements::SkillBox;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::resource::SkillSource;
use crate::interface::theme::InterfaceTheme;
use crate::inventory::Skill;
use crate::GameState;

pub struct HotbarContainer {
    skills: PlainRemote<[Option<Skill>; 10]>,
    state: ContainerState<GameState>,
}

impl HotbarContainer {
    pub fn new(skills: PlainRemote<[Option<Skill>; 10]>) -> Self {
        let elements = {
            let skills = skills.get();

            skills
                .iter()
                .cloned()
                .enumerate()
                .map(|(slot, skill)| {
                    let skill_source = SkillSource::Hotbar {
                        slot: HotbarSlot(slot as u16),
                    };
                    let skill_box = SkillBox::new(
                        skill,
                        skill_source,
                        Box::new(move |mouse_mode| matches!(mouse_mode, MouseInputMode::MoveSkill(source, _) if *source != skill_source)),
                    );

                    skill_box.wrap()
                })
                .collect()
        };

        let state = ContainerState::new(elements);

        Self { skills, state }
    }
}

impl Element<GameState> for HotbarContainer {
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state.state
    }

    fn link_back(&mut self, weak_self: WeakElementCell<GameState>, weak_parent: Option<WeakElementCell<GameState>>) {
        self.state.link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        self.state.is_focusable::<false>()
    }

    fn focus_next(
        &self,
        self_cell: ElementCell<GameState>,
        caller_cell: Option<ElementCell<GameState>>,
        focus: Focus,
    ) -> Option<ElementCell<GameState>> {
        self.state.focus_next::<false>(self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell<GameState>) -> Option<ElementCell<GameState>> {
        self.state.restore_focus(self_cell)
    }

    fn resolve(
        &mut self,
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        let size_bound = &size_bound!(100%, ?);
        self.state
            .resolve(placement_resolver, state, theme_selector, size_bound, ScreenSize::uniform(3.0));
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        if self.skills.consume_changed() {
            let weak_self = self.state.state.self_element.take().unwrap();
            let weak_parent = self.state.state.parent_element.take();

            *self = Self::new(self.skills.clone());
            // important: link back after creating elements, otherwise focus navigation and
            // scrolling would break
            self.link_back(weak_self, weak_parent);

            return Some(ChangeEvent::RESOLVE_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<GameState> {
        match mouse_mode {
            MouseInputMode::MoveSkill(..) | MouseInputMode::None => self.state.hovered_element(mouse_position, mouse_mode, false),
            _ => HoverInformation::Missed,
        }
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        application: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        self.state.render(&mut renderer, application, theme_selector, second_theme);
    }
}
