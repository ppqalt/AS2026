//! Näppäimistökohdistuksen näkyvyys seuraa syöttötapaa, ei itse kohdistusta.
use slint::winit_030::{
    EventResult, WinitWindowAccessor,
    winit::{
        event::{ElementState, TouchPhase, WindowEvent},
        keyboard::{Key, NamedKey},
    },
};

fn keyboard_mode(key: &Key, state: ElementState) -> Option<bool> {
    (state == ElementState::Pressed && *key == Key::Named(NamedKey::Tab)).then_some(true)
}

pub(super) fn bind(window: &slint::Window, mut update: impl FnMut(bool) + 'static) {
    window.on_winit_window_event(move |_, event| {
        let mode = match event {
            WindowEvent::KeyboardInput { event, .. } => {
                keyboard_mode(&event.logical_key, event.state)
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            }
            | WindowEvent::Focused(false) => Some(false),
            WindowEvent::Touch(touch) if touch.phase == TouchPhase::Started => Some(false),
            _ => None,
        };
        if let Some(visible) = mode {
            update(visible);
        }
        // Havaitaan vain syöttötapa. Slint käsittelee tapahtuman normaalisti.
        EventResult::Propagate
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_tab_press_starts_keyboard_focus_display() {
        assert_eq!(
            keyboard_mode(&Key::Named(NamedKey::Tab), ElementState::Pressed),
            Some(true)
        );
        assert_eq!(
            keyboard_mode(&Key::Named(NamedKey::Tab), ElementState::Released),
            None
        );
        for key in [
            Key::Named(NamedKey::Enter),
            Key::Named(NamedKey::Space),
            Key::Character("a".into()),
        ] {
            assert_eq!(keyboard_mode(&key, ElementState::Pressed), None);
        }
    }
}
