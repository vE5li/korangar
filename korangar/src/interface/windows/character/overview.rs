use std::cell::UnsafeCell;
use std::fmt::Display;

use derive_new::new;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, Path, Selector};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

// TODO: Make this more generic and put it into korangar_interface.
pub struct PartialEqDisplaySelector<P, T> {
    path: P,
    last_value: UnsafeCell<Option<T>>,
    text: UnsafeCell<String>,
}

impl<P, T> PartialEqDisplaySelector<P, T> {
    pub fn new(path: P) -> Self {
        Self {
            path,
            last_value: UnsafeCell::default(),
            text: UnsafeCell::default(),
        }
    }
}

impl<P, T> Selector<ClientState, String> for PartialEqDisplaySelector<P, T>
where
    P: Path<ClientState, T>,
    T: Clone + PartialEq + Display + 'static,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a String> {
        // SAFETY
        // `unnwrap` is safe here because the bound of `P` specifies a safe path.
        let value = self.path.follow(state).unwrap();

        unsafe {
            let last_value = &mut *self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| last != value) {
                *self.text.get() = value.to_string();
                *last_value = Some(value.clone());
            }
        }

        unsafe { Some(self.text.as_ref_unchecked()) }
    }
}

pub struct CharacterOverviewWindow<P, L, J> {
    player_name: P,
    base_level: L,
    job_level: J,
}

impl<P, L, J> CharacterOverviewWindow<P, L, J> {
    pub fn new(player_name: P, base_level: L, job_level: J) -> Self {
        Self {
            player_name,
            base_level,
            job_level,
        }
    }
}

impl<P, L, J> CustomWindow<ClientState> for CharacterOverviewWindow<P, L, J>
where
    P: Path<ClientState, String>,
    L: Path<ClientState, usize>,
    J: Path<ClientState, usize>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterOverview)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            text! {
                text: self.player_name,
            },
            text! {
                text: PartialEqDisplaySelector::new(self.base_level),
            },
            text! {
                text: PartialEqDisplaySelector::new(self.job_level),
            },
            button! {
                text: "Inventory",
                event: UserEvent::OpenInventoryWindow,
            },
            button! {
                text: "Equipment",
                event: UserEvent::OpenEquipmentWindow,
            },
            button! {
                text: "Skill tree",
                event: UserEvent::OpenSkillTreeWindow,
            },
            button! {
                text: "Friends",
                event: UserEvent::OpenFriendsWindow,
            },
            button! {
                text: "Menu",
                event: UserEvent::OpenMenuWindow,
            },
        );

        window! {
            title: "Character Overview",
            class: Some(WindowClass::CharacterOverview),
            theme: ClientThemeType::Game,
            elements: elements,
        }
    }
}
