use korangar_components::skill_box;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use ragnarok_packets::HotbarSlot;
use rust_state::{ArrayLookupExt, Context, OptionExt, Path};

use crate::interface::layout::ScreenSize;
use crate::interface::resource::SkillSource;
use crate::interface::windows::{WindowCache, WindowClass};
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

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Hotbar",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            elements: (
                split! {
                    children: std::array::from_fn::<_, N, _>(|slot| {
                        let path = self.skills_path.array_index(slot).unwrapped();

                        skill_box! {
                            skill_path: path,
                            source: SkillSource::Hotbar { slot: HotbarSlot(slot as u16) },
                        }
                    }),
                },
            )
        }
    }
}
