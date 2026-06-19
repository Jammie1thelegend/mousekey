use rdev::{grab, listen, simulate, Key, Button, Event, EventType};

use std::fs;
use log::{LevelFilter};
use env_logger::Builder;

use std::{thread, time};
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::{AtomicBool, Ordering, AtomicI8};

use std::process::{Child, Command};

use std::collections::HashMap;

use std::prelude::v1;

use serde::Deserialize;



#[derive(Deserialize)]
#[derive(Debug)]
pub struct Config {
    pub move_limit: f64,
    pub hold_time: f64,
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
    pub or_x: f64,
    pub or_y: f64,
    pub x_lim_time: f64,
    pub y_lim_time: f64,
    pub move_limit: f64,
    pub hold_time: f64
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
            or_x: 0.0,
            or_y: 0.0,
            x_lim_time: 0.0,
            y_lim_time: 0.0,
            move_limit: config.move_limit,
            hold_time: config.hold_time
        }
    }

    pub fn set_default(&mut self) -> Self {
        self.or_x = 0.0;
        self.or_y = 0.0;
        self.x_lim_time = 0.0;
        self.y_lim_time = 0.0;
        return *self
    }
}



static QUIT_COMBO: AtomicI8 = AtomicI8::new(0);
static PASSTHROUGH: AtomicBool = AtomicBool::new(false);
static STOP_REP: AtomicBool = AtomicBool::new(false);


static CONV_MOUSE_BUTTON: LazyLock<HashMap<Button, Key>> = LazyLock::new(|| HashMap::from([
    (Button::Left, Key::Unknown(0x86)),      //F23
    (Button::Right, Key::Unknown(0x85)),     //F22
    (Button::Middle, Key::Unknown(0x84)),    //F21
    (Button::Unknown(0x05), Key::Unknown(0x83)),        //F20 & button_extra -> might need to change these to "keys" as button may not be recognised
    (Button::Unknown(0x06), Key::Unknown(0x82)),        //F19 & button side
    ]));

static CONV_KEY: LazyLock<HashMap<Key, Key>> = LazyLock::new(|| HashMap::from([
    (Key::CapsLock, Key::Unknown(0x87)),     //F24
    (Key::UpArrow, Key::Unknown(0x81)),       //F18
    (Key::DownArrow, Key::Unknown(0x80)),     //F17
    ]));


static TRACKED_KEYS_STATE: LazyLock<Mutex<HashMap<Key, i8>>> = LazyLock::new(|| Mutex::new(HashMap::from([
    (Key::CapsLock, 0),
    (Key::F5, 0),
    (Key::F7, 0),
    (Key::F8, 0),
    ])));

static TRACKED_BUTTONS_STATE: LazyLock<Mutex<HashMap<Button, i8>>> = LazyLock::new(|| Mutex::new(HashMap::from([
    (Button::Left, 0),
    (Button::Right, 0),
    (Button::Middle, 0),
    (Button::Unknown(0x05), 0),        //button_extra & side -> might need to change these to "keys" as button may not be recognised
    (Button::Unknown(0x06), 0),
    ])));

static MOUSEARROW: LazyLock<Mutex<MouseArrow>> = LazyLock::new(|| Mutex::new(MouseArrow::load_default()));

static KANATA: LazyLock<Mutex<Vec<Child>>> = LazyLock::new(|| Mutex::new(vec![]));


fn main() {


    // initialize logger
    Builder::new()
        .filter_level(LevelFilter::Trace)  // This shows everything
        .init();


    // load config to mousearrow struct
    MouseArrow::load_default();


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

    // look into wintercept driver version

    KANATA.lock().unwrap().push(Command::new("./kanata_windows_tty_winIOv2_x64.exe")
        .arg("--cfg")
        .arg("mousekey.kbd")
        .spawn()
        .unwrap());
    

    thread::sleep(time::Duration::from_millis(3000)); // creates a gap between warning message and program begin

    println!("\nKanata started. Continuing...\n");


    //let (cltx, clrx) = mpsc::channel();

    // capslock/mouse passthrough thread
    /* thread::spawn(move || {
        loop {
            let val = clrx.recv().unwrap();
            if val == 2 {mouse_passthrough_tx.store(true, Ordering::Relaxed);}
            else if mouse_passthrough_tx.load(Ordering::Relaxed) == true {
                mouse_passthrough_tx.store(false, Ordering::Relaxed);
            }
        }
    }); */
    
    
    thread::spawn(move || { 
        loop {
            let _ = listen(listen_callback);
        }
    });



    // main loop

    loop {
        let _ = grab(grab_callback);

         }
    }

fn listen_callback(ev: Event){
    match ev.event_type {
        EventType::KeyPress(k) => {if !(TRACKED_KEYS_STATE.lock().unwrap().contains_key(&k)) {};  process_keyboard(k, true);},
        EventType::KeyRelease(k) => {if !(TRACKED_KEYS_STATE.lock().unwrap().contains_key(&k)) {};  process_keyboard(k, false);},
        _ => {}
    }
}



fn grab_callback(ev: Event) -> Option<Event> {
    match check_mouse_passthrough() {false => {}, true => {return Some(ev)}};
    match ev.event_type {
        EventType::ButtonPress(b) => {if !(TRACKED_BUTTONS_STATE.lock().unwrap().contains_key(&b)) {return Some(ev)};  process_mouse_b(b, true); return v1::None},
        EventType::ButtonRelease(b) => {if !(TRACKED_BUTTONS_STATE.lock().unwrap().contains_key(&b)) {return Some(ev)};  process_mouse_b(b, false); return v1::None},
        EventType::Wheel { delta_x: _, delta_y } => {process_mouse_w(delta_y); return v1::None}
        EventType::MouseMove { x, y } => {process_mouse_m(x, y); return v1::None}
        _ => {return Some(ev)}
    }
}

fn process_keyboard(key: Key, down: bool) {
    let mut tracked_val = match TRACKED_KEYS_STATE.lock().unwrap().get(&key) {Some(k) => {*k}, v1::None => {return}};

    match down {
        true => {if tracked_val == 2 {} else if tracked_val == 1 {tracked_val = 2} else {tracked_val = 1}}
        false => {tracked_val = 0}
    }
    let _ = TRACKED_KEYS_STATE.lock().unwrap().entry(key).and_modify(|val| *val = tracked_val);
    

    if key == Key::F5 || key ==  Key::F7 || key ==  Key::F8 {
        let mut qc = QUIT_COMBO.load(Ordering::Relaxed);
        qc += match tracked_val {0 => {-1}, 1 => {1}, _ => {0; return}};
        if qc < 0 {qc = 0}
        if qc == 3 {
            KANATA.lock().unwrap()[0].kill().expect("WARNING!! Kanata could not be killed!!!");
            panic!("__ ___ FORCE QUIT ___ __");
        }
        QUIT_COMBO.store(qc, Ordering::Relaxed);
        return
    }
    
    //let passthrough: Key = *CONV_KEY.get(&Key::CapsLock).unwrap();
    let passthrough: Key = Key::CapsLock;
    if key == passthrough {//let _ = cltx.send(tracked_val);
        if tracked_val == 2 {PASSTHROUGH.store(true, Ordering::Relaxed);}
        else if PASSTHROUGH.load(Ordering::Relaxed) == true {
            PASSTHROUGH.store(false, Ordering::Relaxed);
    }
}

}

fn process_mouse_b(button: Button, down: bool) {
    /* let mut tracked_val = match TRACKED_BUTTONS_STATE.lock().unwrap().get_mut(&button) {Some(b) => {*b}, v1::None => {return}};

    match down {
        true => {if tracked_val == 2 {} else if tracked_val == 1 {tracked_val = 2} else {tracked_val = 1}}
        false => {tracked_val = 0}
    } */
    *MOUSEARROW.lock().unwrap() = MouseArrow::set_default(&mut MOUSEARROW.lock().unwrap());
    STOP_REP.store(true, Ordering::Relaxed);

    let conv_button = *CONV_MOUSE_BUTTON.get(&button).unwrap();

    match down {
        true => {let _ = simulate(&EventType::KeyPress(conv_button));}
        false => {let _ = simulate(&EventType::KeyRelease(conv_button));}
    }
    return

}

fn process_mouse_w(value: i64) {
    if value == 0 {return}

    let mut dir_code = Key::Return;

    if value > 0 {
        dir_code = Key::UpArrow;
    }
    else if value < 0 {
        dir_code = Key::DownArrow;
    }
    if dir_code != Key::Return {
        let conv_code = match CONV_KEY.get(&dir_code) {Some(code) => {*code}, v1::None => {return}};
        let _ = simulate(&EventType::KeyPress(conv_code));
        let _ = simulate(&EventType::KeyRelease(conv_code));
    }

}

fn process_mouse_m(x: f64, y: f64) {
    let mut mousearrow = MOUSEARROW.lock().unwrap().clone();
    
    if mousearrow.or_x == 0.0 && mousearrow.or_y == 0.0 {
        mousearrow.or_x = x;
        mousearrow.or_y = y;
        *MOUSEARROW.lock().unwrap() = mousearrow;
        return
    }
    let mut rel_x = x- mousearrow.or_x;
    let mut rel_y = mousearrow.or_y - y;

    // "normalise" -> we only want one axis at a time to be near move_limit

    if rel_x.abs() > (mousearrow.move_limit* 0.75) {rel_y = 0.0};
    if rel_y.abs() > (mousearrow.move_limit* 0.75) {rel_x = 0.0};

    if rel_x.abs() >= mousearrow.move_limit {
        /* let mut dir = 1; // only if we need to store current pos
        if rel_x < 0.0 {dir = -1}; */
        let mut key = Key::LeftArrow;
        if rel_x > 0.0 {key = Key::RightArrow};

        mousearrow.x_lim_time = mouse_arrows(key,mousearrow.x_lim_time);
        
    }

    else if rel_y.abs() >= mousearrow.move_limit {
        /* let mut dir = 1; // only if we need to store current pos
        if rel_y < 0.0 {dir = -1}; */
        let mut key = Key::DownArrow;
        if rel_y > 0.0 {key = Key::UpArrow};

        mousearrow.y_lim_time = mouse_arrows(key,mousearrow.y_lim_time);
        
    }
    else {
        STOP_REP.store(true, Ordering::Relaxed);
        mousearrow.x_lim_time = 0.0;
        mousearrow.y_lim_time = 0.0;
    } // stop repeating



    *MOUSEARROW.lock().unwrap() = mousearrow;

}





fn mouse_arrows(key: Key, mut axis_lim_time: f64) -> f64 {
    let hold_time = MOUSEARROW.lock().unwrap().hold_time;



    if axis_lim_time == 0.0 {
        let _ = simulate(&EventType::KeyPress(key));
        let _ = simulate(&EventType::KeyRelease(key));
    }

    if axis_lim_time > hold_time {
        if axis_lim_time < 1000.0 {
            axis_lim_time = 1000.0;
            STOP_REP.store(true, Ordering::Relaxed);
            thread::sleep(time::Duration::from_millis(51));
            STOP_REP.store(false, Ordering::Relaxed);
            create_m_ar_thread(key);
        }
    }

    axis_lim_time += 1.0;
    return axis_lim_time

}

fn create_m_ar_thread(rep_key: Key) {
    thread::spawn(move || { 
        while STOP_REP.load(Ordering::Relaxed) == false {

            thread::sleep(time::Duration::from_millis(50));

            if STOP_REP.load(Ordering::Relaxed) == true || PASSTHROUGH.load(Ordering::Relaxed) == true {STOP_REP.store(false, Ordering::Relaxed); break}

            let _ = simulate(&EventType::KeyPress(rep_key));
            let _ = simulate(&EventType::KeyRelease(rep_key));
        }
    });
}

fn check_mouse_passthrough() -> bool {
    if PASSTHROUGH.load(Ordering::Relaxed) == true {
        *MOUSEARROW.lock().unwrap() = MouseArrow::set_default(&mut *MOUSEARROW.lock().unwrap());
        STOP_REP.store(true, Ordering::Relaxed);
        return true
    }

    return false
}


