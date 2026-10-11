use std::process::{Child, Command};

use crate::{config::Config, mousekeytypes::{RelativeCoord, event_types::{KeyEvent, KeyEventCode, RelativeAxisEvent, RelativeAxisEventCode}}};

    pub fn get_input_devices() {

    }
    pub fn fetch_mouse_events() -> RelativeCoord {
        return RelativeCoord::new(0, 0)
    }
    pub fn fetch_keyboard_events() -> Vec<KeyEvent> {
        return vec![KeyEvent::new(KeyEventCode::None, 1)]
    }
    pub fn reset_cursor(relative_coord: RelativeCoord) {

    }
    pub fn send_key_event(event: KeyEvent) {

    }
    pub fn setup_kanata(emitter_keys: Vec<KeyEventCode>, config: &Config) -> Child {
        
    }