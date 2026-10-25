use thiserror::Error;

use crate::{config::Config, mousekeytypes::event_types::KeyEventCode};


pub const QUIT_KEYS: [KeyEventCode; 3] = [KeyEventCode::F5, KeyEventCode::F7, KeyEventCode::F8];
pub const PASSTHROUGH_KEY: KeyEventCode = KeyEventCode::F13;
pub const EMITTER_KEYS: [KeyEventCode; 4] = [KeyEventCode::Up, KeyEventCode::Down, KeyEventCode::Left, KeyEventCode::Right];
pub const PRIVATE_KANATA_LRLD: KeyEventCode = KeyEventCode::F14;
pub const CFG_PATH: &str = "config.toml";
pub const CFG_DEFAULT: &str = "move_limit = 40\nhold_time = 45\nshow_layer = true";
pub const KANATA_CFG_PATH: &str = "mousekey.kbd";

#[cfg(target_os = "linux")]
pub const KANATA_APPLICATION_PATH: &str = "./kanata_linux_x64";
#[cfg(target_os = "windows")]
pub const KANATA_APPLICATION_PATH: &str = "./kanata_windows_tty_winIOv2_x64.exe";



#[derive(Error, Debug)]
pub enum MouseKeyError {
    #[error("Failed to find a compatible input device. Make sure you are running the application with root priviledges.")]
    NoDeviceError,
}

#[derive(Debug)]
pub enum Dir {
    None,
    Up,
    Down,
    Left,
    Right
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
#[derive(PartialEq)]
#[derive(Debug)]
pub enum HandleKeyboardResult {
    NoOutput,
    ForceQuit,
    StartPassthrough,
    StopPassthrough,
}

#[derive(Debug)]
pub struct PlaceHolderName {
    pub major: MouseKeyAxisInfo,
    pub minor: MouseKeyAxisInfo
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

    pub fn choose_major_axis(mut axes: PlaceHolderName) -> Self {
        if axes.major.val.abs() > ((axes.major.limit as f32 * 0.75) as i32) {axes.minor.val = 0};
        return axes.minor
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

pub mod event_types {
    #[derive(Copy, Clone)]
    #[derive(Debug)]
    #[derive(PartialEq)]
    pub enum KeyEventCode {
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
    pub enum RelativeAxisEventCode {
        None,
        XAxis,
        YAxis,
    }

    pub struct KeyEvent {
        pub code: KeyEventCode, 
        pub val: i32
    }

    impl KeyEvent {
        pub fn new(code: KeyEventCode, val: i32) -> Self {
            let new = KeyEvent {
                code: code,
                val: val
            };
            return new
        }
    }

    pub struct RelativeAxisEvent {
        pub code: RelativeAxisEventCode, 
        pub val: i32
    }

    impl RelativeAxisEvent {
        pub fn new(code: RelativeAxisEventCode, val: i32) -> Self {
            let new = RelativeAxisEvent {
                code: code,
                val: val
            };
            return new
        }
    }

}