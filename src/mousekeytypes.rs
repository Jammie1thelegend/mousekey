use thiserror::Error;

use crate::config::Config;


#[derive(Error, Debug)]
pub enum MouseKeyError {
    #[error("Failed to find a compatible input device. Make sure you are running the application with root priviledges.")]
    NoDeviceError
}

#[derive(Debug)]
pub enum Dir {
    None,
    Up,
    Down,
    Left,
    Right
}


#[derive(Debug)]
pub enum MouseKeyEventType {
    KeyEvent(KeyEvent),
    RelativeAxisEvent(RelativeAxisEvent),
}

#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub enum KeyEvent {
    None,
    Left,
    Right,
    Up,
    Down,
    CapsLock,
    F5,
    F6,
    F7,
    F8,
    F13,
    F14,
    MouseLeft,

}

#[derive(Copy, Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub enum RelativeAxisEvent {
    None,
    XAxis,
    YAxis,
}

#[derive(Debug)]
pub struct MouseKeyEvent {
    pub event: MouseKeyEventType,
    pub value: i32,
}

impl MouseKeyEvent {
    pub fn new(event_type: MouseKeyEventType, event_value: i32) -> MouseKeyEvent {
        let new = MouseKeyEvent {
            event: event_type,
            value: event_value,
        };
        return new
    }
}

#[derive(Copy, Clone)]
#[derive(Debug)]
pub struct RelativeCoord {
    pub x: i32,
    pub y: i32,
}

impl RelativeCoord {
    pub fn new(x: i32, y: i32) -> RelativeCoord {
        let new = RelativeCoord {
            x: x,
            y: y
        };
        return new
    }
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
#[derive(Debug)]
pub enum AxisOutput {
    NoOutput,
    SingleSend,
    Repeat,
}

#[derive(Copy, Clone)]
#[derive(Debug)]
pub struct MouseKeyAxisInfo {
    pub val: i32,
    pub limit: i32,
    pub limit_time: i32,
    pub hold_time: i32,
    pub axis_output: AxisOutput,
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

#[derive(Copy, Clone)]
#[derive(Debug)]
pub struct MouseArrow {
    pub x_axis: MouseKeyAxisInfo,
    pub y_axis: MouseKeyAxisInfo,
}

impl MouseArrow {
    pub fn load_default(config: Config) -> Self {
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