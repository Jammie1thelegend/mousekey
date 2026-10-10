
use std::{thread, time};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use crate::config::Config;
use crate::os_specific;



use crate::mousekeytypes::{AxisOutput, KeyEvent, MouseArrow, MouseKeyAxisInfo, MouseKeyError, MouseKeyEvent, MouseKeyEventType, RelativeAxisEvent, RelativeCoord, Dir};

pub const QUIT_KEYS: [KeyEvent; 3] = [KeyEvent::F5, KeyEvent::F7, KeyEvent::F8];
pub const MOUSEPASSTHROUGH_KEY: KeyEvent = KeyEvent::CapsLock;

static PASSTHROUGH: AtomicBool = AtomicBool::new(false);
static STOP_REP: AtomicBool = AtomicBool::new(false);






fn main() {
    // add error handling (return result not just requested thing)
    let virtual_devices = setup_virtual_devices();
    let kanata = setup_kanata();
    os_specific::get_input_devices();

    //keyboard thread
    thread::spawn(move || {
        let mut quit_combo: i8 = 0;

        loop {
            quit_combo = handle_keyboard(quit_combo);
        }
    });

    //mouse thread
    thread::spawn(move || {
        let mousearrow: MouseArrow = MouseArrow::load_default(Config::load());

        loop {
            mousearrow = handle_mouse(mousearrow);
        }
    });

    MouseKeyEventType::KeyEvent(KeyEvent::Left);
}

fn handle_keyboard(quit_combo: i8) -> i8 {
    for event in os_specific::fetch_keyboard_events() {
        if QUIT_KEYS.contains(event.event) { //need to figure out how to "destructure" MouseKeyEventType
            
        }
    }
    return 1
}


fn handle_mouse(mut mousearrow: MouseArrow) -> MouseArrow {

    if PASSTHROUGH.load(Ordering::Relaxed) == true {
        STOP_REP.store(true, Ordering::Relaxed);
        mousearrow = MouseArrow::set_axes_to_zero(mousearrow);
        return mousearrow
    }

    let mouse_rel_coord = os_specific::fetch_mouse_events();

    mousearrow = process_mouse_move(mouse_rel_coord, mousearrow);

    handle_axis_output(&mousearrow.x_axis, RelativeAxisEvent::XAxis);
    handle_axis_output(&mousearrow.y_axis, RelativeAxisEvent::YAxis);

    os_specific::reset_cursor(mouse_rel_coord);

    return mousearrow
}

fn handle_axis_output(axis_info: &MouseKeyAxisInfo, axis: RelativeAxisEvent) {
    if axis_info.axis_output == AxisOutput::NoOutput {return}

    let event: KeyEvent = determine_arrow_output(axis_info.val, axis);

    if axis_info.axis_output == AxisOutput::SingleSend {
        os_specific::send_key_event(MouseKeyEvent::new(MouseKeyEventType::KeyEvent(event), 1));
        os_specific::send_key_event(MouseKeyEvent::new(MouseKeyEventType::KeyEvent(event), 0));
    }

    if axis_info.axis_output == AxisOutput::Repeat {
        create_m_ar_thread(event);
    }

}

fn determine_arrow_output(axis_val: i32, axis: RelativeAxisEvent) -> KeyEvent {
    let mut event = KeyEvent::None;

    if axis == RelativeAxisEvent::XAxis {
        event = KeyEvent::Left;
        if axis_val > 0 {
            event = KeyEvent::Right;
        }
        return event
    }
    if axis == RelativeAxisEvent::YAxis {
        event = KeyEvent::Down;
        if axis_val > 0 {
            event = KeyEvent::Up;
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

    // "normalise" -> we only want one axis at a time to be near move_limit *** can we make this a function?

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

fn create_m_ar_thread(rep_key: KeyEvent) {
    thread::spawn(move || { 
        while STOP_REP.load(Ordering::Relaxed) == false {

            if STOP_REP.load(Ordering::Relaxed) == true || PASSTHROUGH.load(Ordering::Relaxed) == true {break}

            os_specific::send_key_event(MouseKeyEvent::new(MouseKeyEventType::KeyEvent(rep_key), 1));
            os_specific::send_key_event(MouseKeyEvent::new(MouseKeyEventType::KeyEvent(rep_key), 0));

            thread::sleep(time::Duration::from_millis(50));
        }
    });
}