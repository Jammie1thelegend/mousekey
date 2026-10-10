
use std::process::Command;

use std::{thread, time};
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use std::fs;

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos};
use windows::Win32::UI::Input::KeyboardAndMouse::{self, VIRTUAL_KEY};
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

use enigo::{Enigo, Key, Keyboard, Mouse, Settings, Coordinate};
use enigo::Direction::{Press, Release};

use thiserror::Error;

use serde::Deserialize;

#[derive(Error, Debug)]
pub enum MouseKeyError {
    #[error("Failed to get current mouse position.")]
    FailedGetMousePosition,
}


const KEY_PRESS_VAL_BIG: i16 = -32768;
const KEY_PRESS_VAL_SMALL: i16 = -32767;
const QUIT_KEYS: [VIRTUAL_KEY; 3] = [KeyboardAndMouse::VK_F5, KeyboardAndMouse::VK_F7, KeyboardAndMouse::VK_F8];

static PASSTHROUGH: AtomicBool = AtomicBool::new(false);
static STOP_REP: AtomicBool = AtomicBool::new(false);

static ENIGO: LazyLock<Mutex<Enigo>> = LazyLock::new(|| Mutex::new(Enigo::new(&Settings::default()).unwrap()));


#[derive(Deserialize)]
#[derive(Debug)]
pub struct Config {
    pub move_limit: i32,
    pub hold_time: i32,
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
    pub x: i32,
    pub y: i32,
    pub or_x: i32,
    pub or_y: i32,
    pub x_lim_time: i32,
    pub y_lim_time: i32,
    pub move_limit: i32,
    pub hold_time: i32
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
            x: 0,
            y: 0,
            or_x: 0,
            or_y: 0,
            x_lim_time: 0,
            y_lim_time: 0,
            move_limit: config.move_limit,
            hold_time: config.hold_time
        }
    }

    pub fn set_default(&mut self) -> Self {
        self.x = 0;
        self.y = 0;
        self.or_x = 0;
        self.or_y = 0;
        self.x_lim_time = 0;
        self.y_lim_time = 0;
        return *self
    }
}


fn main() {

    // start kanata
    println!("\nStarting Kanata:\n");

    // look into wintercept driver version

    let mut kanata = Command::new("./kanata_windows_tty_winIOv2_x64.exe")
        .arg("--cfg")
        .arg("mousekey.kbd")
        .spawn()
        .unwrap();
    

    thread::sleep(time::Duration::from_millis(3000)); // creates a gap between warning message and program begin

    println!("\nKanata started. Continuing...\n");


    let mut quit_combo: [bool; 3] = [false; 3];
    let mut mousearrow = MouseArrow::load_default();
    let mut previous_pos  = POINT {x:0, y:0};

    loop {
        quit_combo = check_force_quit(quit_combo);
        if quit_combo == [true; 3] {
            kanata.kill().expect("WARNING!! Kanata could not be killed!!!");
            panic!("__ ___ FORCE QUIT ___ __");
        }

        if check_mouse_passthrough() == true {
            if STOP_REP.load(Ordering::Relaxed) == false {STOP_REP.store(true, Ordering::Relaxed);}
            mousearrow = MouseArrow::set_default(&mut mousearrow);
            thread::sleep(time::Duration::from_millis(10));
            continue
        }

        let mut position = POINT {x:0, y:0};
        match get_mouse_position() {//          compiler thinks position is unused if just a continue
            Ok(pos) => {position = pos}, Err(_) => {let _ = position; continue}
        };

        if position == previous_pos {continue};

        previous_pos = position;

        mousearrow = process_mouse_m(position.x, position.y, mousearrow);

        thread::sleep(time::Duration::from_millis(10));

    }
}

// shameless copy of https://docs.rs/mouse_position/latest/src/mouse_position/impl_windows.rs.html#2-19
fn get_mouse_position() -> Result<POINT, MouseKeyError> {
    let mut position = POINT {x:0, y:0};
    match unsafe {GetCursorPos(&mut position)} {
        Ok(_) => {return Ok(position)}, Err(_) => {return Err(MouseKeyError::FailedGetMousePosition)}
    };
} 

fn check_mouse_passthrough() -> bool {
    let mut passthrough_key_state = vec![];
    unsafe {passthrough_key_state.push(GetAsyncKeyState(KeyboardAndMouse::VK_F13.0 as i32))};

    match passthrough_key_state[0] {KEY_PRESS_VAL_BIG => {return true}, KEY_PRESS_VAL_SMALL => {return true}, _ => {return false}};
}

fn check_force_quit(mut quit_combo: [bool; 3]) -> [bool; 3] {
    let mut force_quit_key_state = vec![];
    for k in QUIT_KEYS {
        unsafe {force_quit_key_state.push(GetAsyncKeyState(k.0 as i32))};
    };

    let mut idx: usize = 0;
    for k in &force_quit_key_state {
        quit_combo[idx] = match *k {KEY_PRESS_VAL_BIG => {true}, KEY_PRESS_VAL_SMALL => {true}, _ => {false}};
        idx += 1;
    }

    return quit_combo
}

fn process_mouse_m(x: i32, y: i32, mut mousearrow: MouseArrow) -> MouseArrow {
    
    if mousearrow.or_x == 0 && mousearrow.or_y == 0 {
        mousearrow.or_x = x;
        mousearrow.or_y = y;
        return mousearrow;
    }
    mousearrow.x += x- mousearrow.or_x;
    mousearrow.y += mousearrow.or_y - y;

    // "normalise" -> we only want one axis at a time to be near move_limit

    if mousearrow.x.abs() > ((mousearrow.move_limit as f32 * 0.75) as i32) {mousearrow.y = 0};
    if mousearrow.y.abs() > ((mousearrow.move_limit as f32 * 0.75) as i32) {mousearrow.x = 0};

    if mousearrow.x.abs() >= mousearrow.move_limit {
        let mut capped_x = -mousearrow.move_limit;

        let mut key = Key::LeftArrow;
        if mousearrow.x > 0 {key = Key::RightArrow; capped_x = mousearrow.move_limit};

        mousearrow.x = capped_x;


        mousearrow.x_lim_time = mouse_arrows(key,mousearrow.x_lim_time, mousearrow.hold_time);
        
    }

    else if mousearrow.y.abs() >= mousearrow.move_limit {
        let mut capped_y = -mousearrow.move_limit;

        let mut key = Key::DownArrow;
        if mousearrow.y > 0 {key = Key::UpArrow; capped_y = mousearrow.move_limit};

        mousearrow.y = capped_y;

        mousearrow.y_lim_time = mouse_arrows(key,mousearrow.y_lim_time, mousearrow.hold_time);
        
    }
    else {
        if STOP_REP.load(Ordering::Relaxed) == false {STOP_REP.store(true, Ordering::Relaxed);}
        mousearrow.x_lim_time = 0;
        mousearrow.y_lim_time = 0;
    }

    //println!("{:?}", mousearrow);

    let _ = ENIGO.lock().unwrap().move_mouse(mousearrow.or_x, mousearrow.or_y, Coordinate::Abs);

    return mousearrow

}





fn mouse_arrows(key: Key, mut axis_lim_time: i32, hold_time: i32) -> i32 {

    if axis_lim_time == 0 {
        let _ = ENIGO.lock().unwrap().key(key, Press);
        let _ = ENIGO.lock().unwrap().key(key, Release);
        //println!("{:?}", key);
    }

    if axis_lim_time > hold_time {
        if axis_lim_time < 1000 {
            axis_lim_time = 1000;
            STOP_REP.store(false, Ordering::Relaxed);
            create_m_ar_thread(key);
            //println!("REPEATING {:?}", key);
        }
    }

    axis_lim_time += 1;
    return axis_lim_time

}

fn create_m_ar_thread(rep_key: Key) {
    thread::spawn(move || { 
        while STOP_REP.load(Ordering::Relaxed) == false {

            if STOP_REP.load(Ordering::Relaxed) == true || PASSTHROUGH.load(Ordering::Relaxed) == true {break}

            let _ = ENIGO.lock().unwrap().key(rep_key, Press);
            let _ = ENIGO.lock().unwrap().key(rep_key, Release);

            thread::sleep(time::Duration::from_millis(50));
        }
    });
}