use std::cell::RefCell;
use std::rc::Rc;

use procedural::*;

use crate::interface::*;

pub struct PacketWindow {
    packets: Rc<RefCell<Vec<PacketEntry>>>,
    cleared: Rc<RefCell<bool>>,
    show_pings: Rc<RefCell<bool>>,
    update: Rc<RefCell<bool>>,
}

impl PacketWindow {

    pub const WINDOW_CLASS: &'static str = "network";

    pub fn new(packets: Rc<RefCell<Vec<PacketEntry>>>) -> Self {

        let cleared = Rc::new(RefCell::new(false));
        let pings = Rc::new(RefCell::new(false));
        let update = Rc::new(RefCell::new(true));

        Self {
            packets,
            cleared,
            show_pings: pings,
            update,
        }
    }
}

impl PrototypeWindow for PacketWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![cell!(PacketView::new(
            self.packets.clone(),
            self.cleared.clone(),
            self.update.clone()
        ))];

        let clear_selector = {

            let packets = self.packets.clone();
            move || !packets.borrow().is_empty()
        };

        let clear_action = {

            let packets = self.packets.clone();
            let cleared = self.cleared.clone();

            move || {

                packets.borrow_mut().clear();
                *cleared.borrow_mut() = true;
            }
        };

        let ping_selector = {

            let show_pings = self.show_pings.clone();
            move |_: &StateProvider| *show_pings.borrow()
        };

        let ping_action = {

            let show_pings = self.show_pings.clone();

            move || {

                let mut show_pings = show_pings.borrow_mut();
                let current_state = *show_pings;
                *show_pings = !current_state;
            }
        };

        let update_selector = {

            let update = self.update.clone();
            move |_: &StateProvider| *update.borrow()
        };

        let update_action = {

            let update = self.update.clone();

            move || {

                let mut update = update.borrow_mut();
                let current_state = *update;
                *update = !current_state;
            }
        };

        let elements = vec![
            Button::default()
                .with_static_text("clear")
                .with_disabled_selector(clear_selector)
                .with_closure(clear_action)
                .with_width(dimension!(33%))
                .wrap(),
            StateButton::default()
                .with_static_text("show pings")
                .with_selector(ping_selector)
                .with_closure(ping_action)
                .with_width(dimension!(33%))
                .wrap(),
            StateButton::default()
                .with_static_text("update")
                .with_selector(update_selector)
                .with_closure(update_action)
                .with_width(dimension!(33%))
                .wrap(),
            cell!(ScrollView::new(elements, constraint!(100%, ?))),
        ];

        Box::from(FramedWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Network".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(300 > 400 < 500, ? < 80%),
        ))
    }
}
