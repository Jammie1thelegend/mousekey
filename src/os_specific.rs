use std::process::{Child, Command};

use crate::mousekeytypes::{KANATA_APPLICATION_PATH, RelativeCoord, event_types::{KeyEvent, KeyEventCode}};



pub fn get_input_devices() {

}

pub fn setup_virtual_devices() {
    
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

pub fn start_kanata(kanata_cfg_path: &str) -> Child {
    
    let kanata = Command::new(KANATA_APPLICATION_PATH)
    .arg("--cfg")
    .arg(kanata_cfg_path)
    .spawn()
    .unwrap();
    
    return kanata
}
