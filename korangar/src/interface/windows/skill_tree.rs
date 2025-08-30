use korangar_components::skill_box;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, VecIndexExt};

use crate::SkillSource;
use crate::interface::windows::WindowClass;
use crate::inventory::Skill;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

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

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // TODO: Just temporary
        const SKILL_TREE_ROWS: usize = 4;
        const SKILL_TREE_COLUMNS: usize = 10;

        window! {
            title: client_state().localization().skill_tree_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: std::array::from_fn::<_, SKILL_TREE_ROWS, _>(|row| {
                split! {
                    gaps: theme().window().gaps(),
                    children: std::array::from_fn::<_, SKILL_TREE_COLUMNS, _>(|column| {
                        let path = self.skills_path.index(row * SKILL_TREE_COLUMNS + column);

                        skill_box! {
                            skill_path: path,
                            source: SkillSource::SkillTree,
                        }
                    }),
                }
            }),
        }
    }
}
