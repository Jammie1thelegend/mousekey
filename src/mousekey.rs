
use std::process::{Child};
use std::fs;
use std::{thread, time};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io;
use std::io::Read;

use log::{LevelFilter};
use env_logger::Builder;


use crate::config::Config;
use crate::os_specific;



use crate::mousekeytypes::{AxisOutput, HandleKeyboardResult, MouseArrow, MouseKeyAxisInfo, MouseKeyError, PlaceHolderName, RelativeCoord};
use crate::mousekeytypes::event_types::{KeyEvent, RelativeAxisEvent, KeyEventCode, RelativeAxisEventCode};
use crate::mousekeytypes::{QUIT_KEYS, PASSTHROUGH_KEY};


static PASSTHROUGH: AtomicBool = AtomicBool::new(false);
static STOP_REP: AtomicBool = AtomicBool::new(false);


pub fn setup(kanata_cfg_path: &str, config: Config) -> Child {

    Builder::new()
    .filter_level(LevelFilter::Trace)
    .init();

    handle_show_layers(kanata_cfg_path, config);

    // \n not needed for some reason, the visual formatting is enough.
    println!("
        **IMPORTANT:
        Force quit keybind: F5+F7+F8
        Mouse passthrough: Hold Caps Lock
        
        WARNING: This program redirects all of the inputs from the devices you select to its 'mousekey' uinput device.
        Please be prepared to force quit the terminal and possibly power down your device before proceeding.
    ");
    
    thread::sleep(time::Duration::from_millis(1000)); // creates a gap between warning message and program begin

    // start kanata
    println!("\nStarting Kanata:\n");



    let kanata: Child = os_specific::start_kanata(kanata_cfg_path);
    
    let stdin = io::stdin();

    let mut line = String::new();

    while line.find("") == None {
        let mut stdin = stdin.lock(); // locking is optional

        // Could also `match` on the `Result` if you wanted to handle `Err` 
        while let Ok(n_bytes) = stdin.read_to_string(&mut line) {
            if n_bytes == 0 {break;}
            println!("READ: {}", line);
            line.clear();
        }
    }

    //logic to match on line to know when kanata has finished starting
    


    thread::sleep(time::Duration::from_millis(3000)); // creates a gap between warning message and program begin

    println!("\nKanata started. Continuing...\n");

    return kanata

        
}


fn handle_show_layers(kanata_cfg_path: &str, config: Config) {
    match fs::File::open(kanata_cfg_path) {
        Ok(file) => {file}, 
        Err(_) => {panic!("could not find/ open {}!", kanata_cfg_path)}
    };

    let mut kanata_cfg_str = match fs::read_to_string(kanata_cfg_path) {
            Ok(raw) => {raw}, Err(_) => {panic!("could not read {}!", kanata_cfg_path)}
        };

    match kanata_cfg_path.find("defcfg") {
        Some(_) => {match kanata_cfg_str.find("log-layer-changes") {
            Some(_) => {},
            None => {kanata_cfg_str = kanata_cfg_str.replace("defcfg\n", "defcfg\nlog-layer-changes no\n");}
        };},
        None => {kanata_cfg_str = "(defcfg\nlog-layer-changes no\n)".to_owned() + &kanata_cfg_str}
    }
    

    let mut kanata_cfg_str_replaced: String = "".to_string();
    
    match config.show_layer {
        false => {
            if kanata_cfg_str.find("log-layer-changes no") == None {
                kanata_cfg_str_replaced = kanata_cfg_str.replace("log-layer-changes yes", "log-layer-changes no");
            }
        },
        true => {
            if kanata_cfg_str.find("log-layer-changes yes") == None  {
                kanata_cfg_str_replaced = kanata_cfg_str.replace("log-layer-changes no", "log-layer-changes yes");
            }
        }
    }

    if kanata_cfg_str_replaced != "".to_string() {
        match fs::write(kanata_cfg_path, kanata_cfg_str_replaced) {
            Ok(_) => {}, Err(_) => {panic!("could not write to {}!", kanata_cfg_path);}
        };
    }

    println!("\nShow layers set to: {}\n", config.show_layer);

}

pub fn handle_keyboard(mut quit_combo: i8) -> (i8, HandleKeyboardResult) {
    for event in os_specific::fetch_keyboard_events() {
        if QUIT_KEYS.contains(&event.code) {
            quit_combo += match event.val {0 => {-1}, 1 => {1}, _ => {0}};
            if quit_combo < 0 {quit_combo = 0}
            if quit_combo == 3 {
                return (quit_combo, HandleKeyboardResult::ForceQuit)
            }
        }

        if event.code == PASSTHROUGH_KEY {
            if event.val == 2 {
                return (quit_combo, HandleKeyboardResult::StartPassthrough)
            }
            else {
                return (quit_combo, HandleKeyboardResult::StopPassthrough)
            }
        }

    }
    return (quit_combo, HandleKeyboardResult::NoOutput)
}

pub fn handle_keyboard_result(code: HandleKeyboardResult, kanata: &mut Child) {
    match code {
        HandleKeyboardResult::NoOutput => {return},

        HandleKeyboardResult::ForceQuit => {
            kanata.kill().expect("WARNING!! Kanata could not be killed!!!");
            panic!("__ ___ FORCE QUIT ___ __");
        }

        HandleKeyboardResult::StartPassthrough => {
            PASSTHROUGH.store(true, Ordering::Relaxed);
        }

        HandleKeyboardResult::StopPassthrough => {
            PASSTHROUGH.store(false, Ordering::Relaxed);
        }

    }
}


pub fn handle_mouse(mut mousearrow: MouseArrow) -> MouseArrow {

    if PASSTHROUGH.load(Ordering::Relaxed) == true {
        STOP_REP.store(true, Ordering::Relaxed);
        mousearrow = MouseArrow::set_axes_to_zero(mousearrow);
        return mousearrow
    }

    let mouse_rel_coord = os_specific::fetch_mouse_events();

    mousearrow = process_mouse_move(mouse_rel_coord, mousearrow);

    handle_axis_output(&mousearrow.x_axis, RelativeAxisEventCode::XAxis);
    handle_axis_output(&mousearrow.y_axis, RelativeAxisEventCode::YAxis);

    os_specific::reset_cursor(mouse_rel_coord);

    return mousearrow
}

fn handle_axis_output(axis_info: &MouseKeyAxisInfo, axis: RelativeAxisEventCode) {
    if axis_info.axis_output == AxisOutput::NoOutput {return}

    let event: KeyEventCode = determine_arrow_output(axis_info.val, axis);

    if axis_info.axis_output == AxisOutput::SingleSend {
        os_specific::send_key_event(KeyEvent::new(event, 1));
        os_specific::send_key_event(KeyEvent::new(event, 0));
    }

    if axis_info.axis_output == AxisOutput::Repeat {
        create_m_ar_thread(event);
    }

}

fn determine_arrow_output(axis_val: i32, axis: RelativeAxisEventCode) -> KeyEventCode {
    let mut event = KeyEventCode::None;

    if axis == RelativeAxisEventCode::XAxis {
        event = KeyEventCode::Left;
        if axis_val > 0 {
            event = KeyEventCode::Right;
        }
        return event
    }
    if axis == RelativeAxisEventCode::YAxis {
        event = KeyEventCode::Down;
        if axis_val > 0 {
            event = KeyEventCode::Up;
        }
        return event
    }

    return event
}


fn process_mouse_move(mouse_rel_coord: RelativeCoord, mousearrow: MouseArrow) -> MouseArrow {
    let mut x_axis: MouseKeyAxisInfo = mousearrow.x_axis;
    let mut y_axis: MouseKeyAxisInfo = mousearrow.y_axis;

    x_axis.val += mouse_rel_coord.x;
    y_axis.val += mouse_rel_coord.y;

    // "normalise" -> we only want one axis at a time to be near move_limit *** needs proper naming and needs better functions

    x_axis = MouseKeyAxisInfo::choose_major_axis(PlaceHolderName{major: y_axis, minor: x_axis});
    y_axis = MouseKeyAxisInfo::choose_major_axis(PlaceHolderName{major: x_axis, minor: y_axis});

    if x_axis.val.abs() > ((x_axis.limit as f32 * 0.75) as i32) {y_axis.val = 0};
    if y_axis.val.abs() > ((y_axis.limit as f32 * 0.75) as i32) {x_axis.val = 0};

    if x_axis.val.abs() > x_axis.limit {
        x_axis = handle_axis_limit(x_axis);
    }
    else {x_axis.limit_time = 0; x_axis.axis_output = AxisOutput::NoOutput;}

    if y_axis.val.abs() > y_axis.limit {
        y_axis = handle_axis_limit(y_axis);
    }
    else {y_axis.limit_time = 0; y_axis.axis_output = AxisOutput::NoOutput;}

    return MouseArrow {x_axis, y_axis}

}

fn handle_axis_limit(mut axis_info: MouseKeyAxisInfo) -> MouseKeyAxisInfo {

    axis_info.val = MouseKeyAxisInfo::clamp_val(axis_info);

    axis_info.axis_output = get_axis_output(axis_info);

    axis_info.limit_time += 1;

    return axis_info

}

fn get_axis_output(axis_info: MouseKeyAxisInfo) -> AxisOutput {

    let mut limit_result = AxisOutput::NoOutput;

    if axis_info.limit_time == 0 {
        limit_result = AxisOutput::SingleSend;
        return limit_result
    }

    if axis_info.limit_time > axis_info.hold_time || axis_info.axis_output != AxisOutput::Repeat {
        limit_result = AxisOutput::Repeat;
        return limit_result
    }

    return limit_result

}

fn create_m_ar_thread(rep_key: KeyEventCode) {
    thread::spawn(move || { 
        while STOP_REP.load(Ordering::Relaxed) == false {

            if STOP_REP.load(Ordering::Relaxed) == true || PASSTHROUGH.load(Ordering::Relaxed) == true {break}

            os_specific::send_key_event(KeyEvent::new(rep_key, 1));
            os_specific::send_key_event(KeyEvent::new(rep_key, 0));

            thread::sleep(time::Duration::from_millis(50));
        }
    });
}