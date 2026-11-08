use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Clipboard access is lazy and only invoked by an explicit paste shortcut in a
/// focused text field. Never log clipboard contents or errors containing them.
pub(crate) fn read_clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

pub(crate) fn is_paste_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    paste_shortcut_for_platform(key, modifiers, cfg!(target_os = "macos"))
}

fn paste_shortcut_for_platform(key: &Key, modifiers: ModifiersState, macos: bool) -> bool {
    if modifiers.alt_key() {
        return false;
    }
    let command = if macos {
        modifiers.super_key() && !modifiers.control_key()
    } else {
        modifiers.control_key() && !modifiers.super_key()
    };
    matches!(key, Key::Character(text) if text.eq_ignore_ascii_case("v")) && command
        || matches!(key, Key::Named(NamedKey::Insert)) && modifiers.shift_key() && !modifiers.control_key() && !modifiers.super_key()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_shortcuts_respect_platform_modifiers() {
        let v = Key::Character("v".into());
        assert!(paste_shortcut_for_platform(&v, ModifiersState::CONTROL, false));
        assert!(paste_shortcut_for_platform(
            &Key::Character("V".into()),
            ModifiersState::CONTROL | ModifiersState::SHIFT,
            false
        ));
        assert!(!paste_shortcut_for_platform(&v, ModifiersState::empty(), false));
        assert!(!paste_shortcut_for_platform(
            &v,
            ModifiersState::CONTROL | ModifiersState::ALT,
            false
        ));
        assert!(!paste_shortcut_for_platform(&v, ModifiersState::SUPER, false));
        assert!(paste_shortcut_for_platform(&v, ModifiersState::SUPER, true));
        assert!(!paste_shortcut_for_platform(&v, ModifiersState::CONTROL, true));
        assert!(!paste_shortcut_for_platform(
            &Key::Character("c".into()),
            ModifiersState::CONTROL,
            false
        ));
    }

    #[test]
    fn shift_insert_uses_clipboard_without_command_or_alt() {
        let insert = Key::Named(NamedKey::Insert);
        assert!(paste_shortcut_for_platform(&insert, ModifiersState::SHIFT, false));
        assert!(!paste_shortcut_for_platform(&insert, ModifiersState::empty(), false));
        assert!(!paste_shortcut_for_platform(
            &insert,
            ModifiersState::SHIFT | ModifiersState::CONTROL,
            false
        ));
        assert!(!paste_shortcut_for_platform(
            &insert,
            ModifiersState::SHIFT | ModifiersState::ALT,
            false
        ));
    }
}
