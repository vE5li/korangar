use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, Path};

// use crate::interface::elements::HotbarContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::inventory::Skill;
use crate::state::{ClientState, ClientThemeType};

pub struct HotbarWindow<P, const N: usize> {
    path: P,
}

impl<P, const N: usize> HotbarWindow<P, N> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P, const N: usize> CustomWindow<ClientState> for HotbarWindow<P, N>
where
    P: Path<ClientState, [Option<Skill>; N]>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Hotbar)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (text! {
            text: "Fancy Hotbar",
        },);

        window! {
            title: "Hotbar",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            elements: elements,
        }
    }
}
