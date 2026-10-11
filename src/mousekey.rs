
use std::process::{Child};
use std::{thread, time};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::Config;
use crate::os_specific;



use crate::mousekeytypes::{AxisOutput, HandleKeyboardResult, MouseArrow, MouseKeyAxisInfo, MouseKeyError, PlaceHolderName, RelativeCoord};
use crate::mousekeytypes::event_types::{KeyEvent, RelativeAxisEvent, KeyEventCode, RelativeAxisEventCode};

const QUIT_KEYS: [KeyEventCode; 3] = [KeyEventCode::F5, KeyEventCode::F7, KeyEventCode::F8];
const PASSTHROUGH_KEY: KeyEventCode = KeyEventCode::F13;
const EMITTER_KEYS: [KeyEventCode; 4] = [KeyEventCode::Up, KeyEventCode::Down, KeyEventCode::Left, KeyEventCode::Right];
const PRIVATE_KANATA_LRLD: KeyEventCode = KeyEventCode::F14;


static PASSTHROUGH: AtomicBool = AtomicBool::new(false);
static STOP_REP: AtomicBool = AtomicBool::new(false);



fn main() {
    // add error handling (return result not just requested thing)
    let config: Config = Config::load();

    let kanata: Child = os_specific::setup_kanata(vec![PRIVATE_KANATA_LRLD], &config);

    os_specific::get_input_devices();

    //keyboard thread
    let mut keyboard_thr_kanata = kanata;
    thread::spawn(move || {
        let mut quit_combo: i8 = 0;
        let mut result: HandleKeyboardResult;

        loop {
            (quit_combo, result) = handle_keyboard(quit_combo);
            handle_keyboard_result(result, &mut keyboard_thr_kanata);
            
        }
    });


    //mouse thread
    let mouse_thr_config = config.clone();
    thread::spawn(move || {
        let mut mousearrow: MouseArrow = MouseArrow::load_default(mouse_thr_config);

        loop {
            mousearrow = handle_mouse(mousearrow);
        }
    });
}

fn handle_keyboard(mut quit_combo: i8) -> (i8, HandleKeyboardResult) {
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

fn handle_keyboard_result(code: HandleKeyboardResult, kanata: &mut Child) {
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


fn handle_mouse(mut mousearrow: MouseArrow) -> MouseArrow {

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