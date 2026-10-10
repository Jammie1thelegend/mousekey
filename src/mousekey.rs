use evdev::{
    EventType, InputEvent, KeyCode,
};
use std::fs;

use std::{thread, time};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use thiserror::Error;

use serde::Deserialize;


#[derive(Error, Debug)]
pub enum MouseKeyError {
    #[error("Failed to find a compatible input device. Make sure you are running the application with root priviledges.")]
    NoDeviceError
}


#[derive(PartialEq)]
#[derive(Debug)]
enum AxisOutput {
    NoOutput,
    SingleSend,
    Repeat,
}

impl Copy for AxisOutput {}

impl Clone for AxisOutput {
    fn clone(&self) -> Self {
        *self
    }
}


pub const QUIT_KEYS: [u16; 3] = [KeyCode::KEY_F5.0, KeyCode::KEY_F7.0, KeyCode::KEY_F8.0];

static PASSTHROUGH: AtomicBool = AtomicBool::new(false);
static STOP_REP: AtomicBool = AtomicBool::new(false);


#[derive(Debug)]
pub struct MouseKeyAxisInfo {
    pub val: i32,
    pub limit: i32,
    pub limit_time: i32,
    pub hold_time: i32,
    pub axis_output: AxisOutput,
}

impl Copy for MouseKeyAxisInfo {}

impl Clone for MouseKeyAxisInfo {
    fn clone(&self) -> Self {
        *self
    }
}

impl MouseKeyAxisInfo {
    pub fn new(val: i32, limit: i32, limit_time: i32, hold_time: i32, axis_output: AxisOutput) -> Self {
        let new = MouseKeyAxisInfo {
            val: val,
            limit: limit,
            limit_time: limit_time,
            hold_time: hold_time,
            axis_output: axis_output,
        };

        return new
    }

    pub fn vals_to_zero(mut self) -> Self {
        self.val = 0;
        self.limit_time = 0;
        return self
    }

    pub fn clamp_val(self) -> i32 {
    let mut clamped_val = -self.limit;

    if self.val > 0 {clamped_val = self.limit};

    return clamped_val
    }
}


#[derive(Deserialize)]
#[derive(Debug)]
pub struct Config {
    pub move_limit: i32,
    pub hold_time: i32,
    pub show_layer: bool,
}

impl Config {
    pub fn load() -> Self {
        let path = "config.toml";
        let default_config = "move_limit = 40\nhold_time = 45";
        match fs::File::open(path) {
            Ok(file) => {file}, 
            Err(_) => {fs::write(path, default_config).unwrap(); fs::File::open(path).unwrap()}
        };

        let config_raw = match fs::read_to_string(path) {
            Ok(raw) => {raw}, Err(_) => {fs::write(path, default_config).unwrap(); fs::read_to_string(path).unwrap()}
        };

    // add error handling
        let config: Self = toml::from_str(&config_raw).unwrap();
        return config
    }
}


#[derive(Debug)]
pub struct MouseArrow {
    pub x_axis: MouseKeyAxisInfo,
    pub y_axis: MouseKeyAxisInfo,
}

impl Copy for MouseArrow {}

impl Clone for MouseArrow {
    fn clone(&self) -> Self {
        *self
    }
}

impl MouseArrow {
    pub fn load_default() -> Self {
        let config = Config::load();
        Self {
            x_axis: MouseKeyAxisInfo::new(0, config.move_limit, 0, config.hold_time, AxisOutput::NoOutput),
            y_axis: MouseKeyAxisInfo::new(0, config.move_limit, 0, config.hold_time, AxisOutput::NoOutput),
        }
    }

    pub fn set_axes_to_zero(mut self) -> Self {
        self.x_axis = MouseKeyAxisInfo::vals_to_zero(self.x_axis);
        self.y_axis = MouseKeyAxisInfo::vals_to_zero(self.y_axis);
        return self
    }
}

fn main() {
    // add error handling (return result not just requested thing)
    let virtual_devices = setup_virtual_devices();
    let kanata = setup_kanata();
    get_input_devices();

    //keyboard thread
    thread::spawn(move || {
        handle_keyboard();
    });

    //mouse thread
    thread::spawn(move || {
        handle_mouse();
    });
}






fn process_mouse_m(x: i32, y: i32, mousearrow: MouseArrow) -> MouseArrow {
    let mut x_axis = mousearrow.x_axis;
    let mut y_axis = mousearrow.y_axis;

    x_axis.val += x;
    y_axis.val += y;

    // "normalise" -> we only want one axis at a time to be near move_limit

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

fn create_m_ar_thread(rep_key: KeyCode, rettx: Sender<InputEvent>) {
    thread::spawn(move || { 
        while STOP_REP.load(Ordering::Relaxed) == false {

            if STOP_REP.load(Ordering::Relaxed) == true || PASSTHROUGH.load(Ordering::Relaxed) == true {break}

            let _ = rettx.send(InputEvent::new(EventType::KEY.0, rep_key.0, 1));
            let _ = rettx.send(InputEvent::new(EventType::KEY.0, rep_key.0, 0));

            thread::sleep(time::Duration::from_millis(50));
        }
    });
}