
mod mousekeytypes;
mod mousekey;
mod config;
mod os_specific;

use std::thread;

use config::Config;
use mousekeytypes::{CFG_DEFAULT, CFG_PATH, KANATA_CFG_PATH};
use mousekeytypes::{HandleKeyboardResult, MouseArrow};


fn main() {
    // add error handling (return result not just requested thing)
    let config: Config = Config::load(CFG_PATH, CFG_DEFAULT);
    let kanata = mousekey::setup(KANATA_CFG_PATH, config);

    os_specific::get_input_devices();

    os_specific::setup_virtual_devices();

    //keyboard thread
    let mut keyboard_thr_kanata = kanata;
    thread::spawn(move || {
        let mut quit_combo: i8 = 0;
        let mut result: HandleKeyboardResult;

        loop {
            (quit_combo, result) = mousekey::handle_keyboard(quit_combo);
            mousekey::handle_keyboard_result(result, &mut keyboard_thr_kanata);
            
        }
    });


    //mouse thread
    let mouse_thr_config = config.clone();
    thread::spawn(move || {
        let mut mousearrow: MouseArrow = MouseArrow::load_default(mouse_thr_config);

        loop {
            mousearrow = mousekey::handle_mouse(mousearrow);
        }
    });
}