use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::Path;

use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::inventory::Skill;
use crate::state::{ClientState, ClientThemeType};

pub struct SkillTreeWindow<P> {
    skills_path: P,
}

impl<P> SkillTreeWindow<P> {
    pub fn new(skills_path: P) -> Self {
        Self { skills_path }
    }
}

impl<P> CustomWindow<ClientState> for SkillTreeWindow<P>
where
    P: Path<ClientState, Vec<Skill>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::SkillTree)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Skill tree",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            closable: true,
            elements: ()
        }
    }
}
