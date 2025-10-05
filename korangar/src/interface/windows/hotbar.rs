use korangar_components::skill_box;
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::HotbarSlot;
use rust_state::{ArrayLookupExt, OptionExt, Path};

use crate::interface::resource::SkillSource;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::skills::{LearnableSkill, LearnedSkill, LearnedSkillPath};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct HotbarWindow<A, B, const N: usize> {
    hotbar_path: A,
    skills_path: B,
}

impl<A, B, const N: usize> HotbarWindow<A, B, N> {
    pub fn new(hotbar_path: A, skills_path: B) -> Self {
        Self { hotbar_path, skills_path }
    }
}

impl<A, B, const N: usize> CustomWindow<ClientState> for HotbarWindow<A, B, N>
where
    A: Path<ClientState, [Option<LearnableSkill>; N]>,
    B: Path<ClientState, Vec<LearnedSkill>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hotbar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().hotbar_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: std::array::from_fn::<_, N, _>(|slot| {
                        let learnable_skill_path = self.hotbar_path.array_index(slot).unwrapped();
                        let learned_skill_path = LearnedSkillPath::new(learnable_skill_path, self.skills_path);

                        skill_box! {
                            learnable_skill_path,
                            learned_skill_path,
                            source: SkillSource::Hotbar { slot: HotbarSlot(slot as u16) },
                        }
                    }),
                },
            )
        }
    }
}
