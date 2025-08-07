use korangar_components::skill_box;
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::HotbarSlot;
use rust_state::{ArrayLookupExt, OptionExt, Path};

use crate::interface::components::skill_box::SkillBoxHandler;
use crate::interface::resource::SkillSource;
use crate::interface::windows::WindowClass;
use crate::inventory::Skill;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct HotbarWindow<P, const N: usize> {
    skills_path: P,
}

impl<P, const N: usize> HotbarWindow<P, N> {
    pub fn new(path: P) -> Self {
        Self { skills_path: path }
    }
}

impl<P, const N: usize> CustomWindow<ClientState> for HotbarWindow<P, N>
where
    P: Path<ClientState, [Option<Skill>; N]>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hotbar)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Hotbar",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: std::array::from_fn::<_, N, _>(|slot| {
                        let path = self.skills_path.array_index(slot).unwrapped();

                        skill_box! {
                            skill_path: path,
                            handler: SkillBoxHandler::new(path, SkillSource::Hotbar { slot: HotbarSlot(slot as u16) }),
                        }
                    }),
                },
            )
        }
    }
}
