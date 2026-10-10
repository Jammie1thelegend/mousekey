
mod mousekeytypes;
//mod linux;
mod mousekey;
mod config;


mod os_specific {
    use crate::mousekeytypes::{KeyEvent, MouseKeyEvent, MouseKeyEventType, RelativeCoord};

    pub fn get_input_devices() {

    }
    pub fn fetch_mouse_events() -> RelativeCoord {
        return RelativeCoord::new(0, 0)
    }
    pub fn fetch_keyboard_events() -> Vec<MouseKeyEvent> {
        return vec![MouseKeyEvent::new(MouseKeyEventType::KeyEvent(KeyEvent::None), 0)]
    }
    pub fn reset_cursor(relative_coord: RelativeCoord) {

    }
    pub fn send_key_event(event: MouseKeyEvent) {

    }
}


fn main() {
    //let _ = linux::main();
}