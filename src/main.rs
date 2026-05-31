use evdev::{
    AttributeSet, AttributeSetRef, Device, EventType, InputEvent, KeyCode, RelativeAxisCode, uinput::VirtualDevice
};
use std::fs;
use thiserror::Error;
use log::{info, warn, error, LevelFilter};
use env_logger::Builder;

use std::{thread, time};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const VDEVICE_NAME: &str = "mousekey";


#[derive(Error, Debug)]
pub enum MouseKeyError {
    #[error("Failed to find a compatible input device. Make sure you are running the application with root priviledges.")]
    NoDeviceError,

    #[error("Failed to read a mouse input")]
    FailedToReadInput,
}
#[derive(Debug)]
struct MouseArrow {
    x: i32,
    y: i32,
    x_lim_time: i32,
    y_lim_time: i32,
    move_limit: i32,
    hold_time: i32
}

impl Copy for MouseArrow {}

impl Clone for MouseArrow {
    fn clone(&self) -> Self {
        *self
    }
}

fn main() -> Result<(), MouseKeyError> {

    // initialize logger
    Builder::new()
        .filter_level(LevelFilter::Trace)  // This shows everything
        .init();


    // \n not needed for some reason, the visual formatting is enough.
    println!("
        **IMPORTANT:
        Force quit keybind: F5+F7+F8
        Mouse passthrough: Hold Caps Lock
        
        WARNING: This program redirects all of the inputs from the devices you select to its 'mousekey' uinput device.
        Please be prepared to force quit the terminal and possibly power down your device before proceeding.
    ");
    
    thread::sleep(time::Duration::from_millis(1000)); // creates a gap between warning message and program begin

    // select mouse device
    let mut mouse = select_input_device(
        EventType::RELATIVE, 
        RelativeAxisCode::REL_X, 
        vec![KeyCode::BTN_LEFT], 
        "mouse"
    ).unwrap();

    //select keyboard device
    let mut keyboard = select_input_device(
        EventType::KEY, 
        RelativeAxisCode::REL_MISC, 
        vec![KeyCode::KEY_F5, KeyCode::KEY_F7, KeyCode::KEY_F8], 
        "keyboard"
    ).unwrap();
    
    thread::sleep(time::Duration::from_millis(100)); // gives enter key time to return to 0


    // set up merged uinput device
    let mut mousekey = create_mousekey(
        VDEVICE_NAME, 
        keyboard.supported_keys().unwrap(), 
        mouse.supported_keys().unwrap(), 
        mouse.supported_relative_axes().unwrap()
    ).unwrap();

    // thread send events to main loop
    let (ktx, rx) = mpsc::channel(); // for keyboard thread
    let mtx = ktx.clone(); // for mouse thread
    let rtx = ktx.clone(); // for arrow key repeat thread
    let mptx = ktx.clone(); // for mouse passthrough/ capslock thread

    let mouse_passthrough_tx = Arc::new(AtomicBool::new(false));
    let mouse_passthrough_rx = mouse_passthrough_tx.clone();
    let mouse_passthrough_ar = mouse_passthrough_tx.clone();
    let (cltx, clrx) = mpsc::channel();

    //keyboard thread
    thread::spawn(move || {
        let _ = keyboard.grab();
        let mut quit_combo = 0;
        loop{
            for ev in keyboard.fetch_events().unwrap(){
                let ev_code: u16 = ev.code();
                if ev_code == KeyCode::KEY_F5.0 || ev_code ==  KeyCode::KEY_F7.0 || ev_code ==  KeyCode::KEY_F8.0 {
                    quit_combo += match ev.value() {0 => {-1}, 1 => {1}, _ => {0}};
                    if quit_combo < 0 {quit_combo = 0}
                    if quit_combo == 3 {panic!("__ ___ FORCE QUIT ___ __");}
                }

                if ev_code == KeyCode::KEY_CAPSLOCK.0 {let _ = cltx.send(ev.value());}

                ktx.send(ev).unwrap()
            }
        }
    });

    // capslock/mouse passthrough thread, so I can thread::sleep before sending capslock(val = 0)
    thread::spawn(move || {
        loop {
            let val = clrx.recv().unwrap();
            if val == 2 {mouse_passthrough_tx.store(true, Ordering::Relaxed);}
            else if mouse_passthrough_tx.load(Ordering::Relaxed) == true {
                mouse_passthrough_tx.store(false, Ordering::Relaxed);
                let _ = mptx.send(InputEvent::new(EventType::KEY.0, KeyCode::KEY_CAPSLOCK.0, 1));
                let _ = mptx.send(InputEvent::new(EventType::KEY.0, KeyCode::KEY_CAPSLOCK.0, 0));
            }
        }
    });


    //mouse thread
    let (retx, rerx) = mpsc::channel(); // send keycode to arrow repeat thread
    let stop_rep = Arc::new(AtomicBool::new(false));
    let stop_rep_ar = Arc::clone(&stop_rep);
    thread::spawn(move || {

        // appease the match logic; it does not approve of a match arm being RelativeAxisCode::REL_X.0, must be a constant
        const REL_X_CODE: u16 = RelativeAxisCode::REL_X.0;
        const REL_Y_CODE: u16 = RelativeAxisCode::REL_Y.0;

        let move_limit = 40;
        let hold_time = 45;

        let mousearrow_defaults = MouseArrow {
        x: 0,
        y: 0,
        x_lim_time: 0,
        y_lim_time: 0,
        move_limit: move_limit,
        hold_time: hold_time
        };


        let mut mousearrow = MouseArrow {
        x: 0,
        y: 0,
        x_lim_time: 0,
        y_lim_time: 0,
        move_limit: move_limit,
        hold_time: hold_time
        };

        let _ = mouse.grab();

        loop{
            for ev in mouse.fetch_events().unwrap(){
                let mut events: Vec<InputEvent> = vec![];
                let mut rep: KeyCode = KeyCode::KEY_10CHANNELSDOWN;

                if mouse_passthrough_rx.load(Ordering::Relaxed) == true || 
                (!(ev.event_type() == EventType::RELATIVE) && !(ev.event_type() == EventType::SYNCHRONIZATION)) {

                    mousearrow = mousearrow_defaults;
                    stop_rep.store(true, Ordering::Relaxed);
                    mtx.send(ev).unwrap();
                    continue
                };

                if ev.value() == 0 {continue}

                match ev.code() {
                    REL_X_CODE => {
                        mousearrow.x += ev.value();
                        if mousearrow.x.abs() > (mousearrow.move_limit as f64 * 0.75) as i32 {mousearrow.y = 0};
                        if mousearrow.x.abs() >= mousearrow.move_limit {
                            let mut dir = 1; //defaults to right, positive movement and KEY_RIGHT
                            if ev.value() < 0 {dir = -1};
                            mousearrow.x = mousearrow.move_limit*dir;

                            let dir_bool = match dir {1 => {true}, -1 => {false}, _ => {false}};

                            let vert = false;

                            (events, mousearrow.x_lim_time, rep) = mouse_arrows(
                                vert, dir_bool, mousearrow.x_lim_time, mousearrow.hold_time
                                );
                            
                        }
                        else {
                            if mousearrow.x_lim_time >= mousearrow.hold_time {
                               rep = KeyCode::KEY_10CHANNELSUP
                            }

                            mousearrow.x_lim_time = 0; 
                        }
                    },
                    REL_Y_CODE => {
                        mousearrow.y += ev.value();
                        if mousearrow.y.abs() > (mousearrow.move_limit as f64 * 0.75) as i32 {mousearrow.x = 0};
                        if mousearrow.y.abs() >= mousearrow.move_limit {
                            let mut dir = 1; //defaults to down, positive movement and KEY_DOWN
                            if ev.value() < 0 {dir = -1};
                            mousearrow.y = mousearrow.move_limit*dir;

                            let dir_bool = match dir {1 => {true}, -1 => {false}, _ => {false}};

                            let vert = true;

                            (events, mousearrow.y_lim_time, rep) = mouse_arrows(
                                vert, dir_bool, mousearrow.y_lim_time, mousearrow.hold_time
                                );
                            
                        }
                        else {
                            if mousearrow.y_lim_time >= mousearrow.hold_time {
                               rep = KeyCode::KEY_10CHANNELSUP
                            }

                            mousearrow.y_lim_time = 0; 
                        }
                    },

                    _ => {events.push(ev);}
                }

                //println!("{:?}", mousearrow);

                if rep != KeyCode::KEY_10CHANNELSDOWN {
                    if rep == KeyCode::KEY_10CHANNELSUP{
                        stop_rep.store(true, Ordering::Relaxed);
                    }
                    else {
                        stop_rep.store(false, Ordering::Relaxed);
                        let _ = retx.send(rep);
                    }
                }

                for e in events {mtx.send(e).unwrap()};
            }
        }
    });

    
    thread::spawn(move || { // arrow key repeat thread
        loop{
            let mut key: KeyCode = rerx.recv().unwrap();

            while key != KeyCode::KEY_10CHANNELSUP || key != KeyCode::KEY_10CHANNELSDOWN {

                thread::sleep(time::Duration::from_millis(50));

                if stop_rep_ar.load(Ordering::Relaxed) == true || mouse_passthrough_ar.load(Ordering::Relaxed) == true {stop_rep_ar.store(false, Ordering::Relaxed); key = KeyCode::KEY_10CHANNELSUP; break}

                let _ = rtx.send(InputEvent::new(EventType::KEY.0, key.0, 1));
                let _ = rtx.send(InputEvent::new(EventType::KEY.0, key.0, 0)); 
            }
        }
    });

    // re-emits events through unified input device
    loop {
        let input: InputEvent = rx.recv().unwrap();

        let _ = &mousekey.emit(&[input]);


        //println!("{:?}", input);
         }
    }






fn select_input_device(filter_evtype: EventType, filter_rel: RelativeAxisCode, filter_keys: Vec<KeyCode>, devname:&str)-> Result<Device, MouseKeyError> {
    // find all input devices that can be used as a specific type of device

    let dev_input_paths: Vec<_> = fs::read_dir("/dev/input/by-id")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.path().into_os_string().to_str().map(String::from)).collect();

    let mut devices: Vec<Device> = Vec::new();
    let mut paths:Vec<String> = Vec::new();
    for p in dev_input_paths {
        let d = Device::open(&p).ok().filter(|device| device.supported_events().contains(filter_evtype));
        match d {
            Some(d) => {
                if filter_rel != RelativeAxisCode::REL_MISC {
                    if d.supported_relative_axes().map_or(false, |axes| axes.contains(filter_rel)) == false {continue}
                }
                if d.supported_keys().map_or(false, |keys| {
                    let mut correct_keys = 0;
                    for fk in &filter_keys {if keys.contains(*fk) {correct_keys += 1}}; // borrows then dereferences, just pleasing the compiler
                    if correct_keys == filter_keys.len() {true}
                    else {false}
                    }
                ) == false {continue}

                devices.push(d); 
                paths.push(p);

                },
            _ => continue
        }
    }
    
    if devices.is_empty() {
        error!("{}", MouseKeyError::NoDeviceError);
        return Err(MouseKeyError::NoDeviceError);
    }
    if devices.len() == 1 {warn!("Only one compatible {} found! ({})", devname, devices[0].name().unwrap_or("Unknown Device"));}

    // await user input to confirm device is at least able to to required functions, despite the fact that it should be able to (some devices are not real)
    if filter_rel == RelativeAxisCode::REL_MISC {
        println!("Awaiting input to choose device, please do the following inputs on your {}: press {:?}. (Does not need to happen at the same time)", devname, filter_keys);
    }
    else {
        println!("Awaiting input to choose device, please do the following inputs on your {}: move {:?}, and press {:?}. (Does not need to happen at the same time)", devname, filter_rel, filter_keys);
    }

    let (dftx, dfrx) = mpsc::channel(); // device found channel
    let dev_found = Arc::new(AtomicBool::new(false));
    for p in paths{
        let dftx_clone = dftx.clone();
        let dev_found_clone = dev_found.clone();
        let mut dev = Device::open(&p).unwrap();
        let mut req_rel = filter_rel.clone().0;
        let mut req_keys: Vec<KeyCode> = filter_keys.clone();
        thread::spawn(move || {
            let req_num = req_keys.len() as i32 + 1; // +1 is req_rel's length
            let mut req_count = 0;

            if req_rel == RelativeAxisCode::REL_MISC.0 {req_count += 1};

            let path_perm = p.clone();

            'thread_loop: loop {
                for ev in dev.fetch_events().unwrap() {
                    if dev_found_clone.load(Ordering::Acquire) == true {break 'thread_loop}


                    let path_temp = path_perm.clone();
                    let ev_code = ev.code();

                    if ev.event_type() == EventType::RELATIVE {
                        if req_rel != RelativeAxisCode::REL_MISC.0 {if ev_code == req_rel {req_count += 1; println!("{:?} moved", RelativeAxisCode(req_rel)); req_rel = RelativeAxisCode::REL_MISC.0;}}
                        
                    }
                    if ev.event_type() == EventType::KEY {
                        let mut rem_req_keys: Vec<KeyCode> = vec![];

                        for rk in &req_keys {
                            if ev_code == rk.0 {req_count += 1; println!("{:?} pressed", rk); continue};
                            rem_req_keys.push(*rk); // * dereferences
                        }
                        req_keys = rem_req_keys;
                    }

                    if req_count == req_num {let _ = dftx_clone.send(path_temp); break 'thread_loop}

                }
            }
        });

    }

    let path: String = dfrx.recv().unwrap().to_string();

    dev_found.store(true, Ordering::Release);
    for mut d in devices {
        let _ = d.send_events(&[InputEvent::new(EventType::KEY.0, filter_keys[0].0, 1)]);
        let _ = d.send_events(&[InputEvent::new(EventType::KEY.0, filter_keys[0].0, 0)]);
    }
    
    
    let input_device = Device::open(path).unwrap();
    info!("Using \"{}\" as {} input device", input_device.name().unwrap_or("Unknown Device"), devname);
    Ok(input_device)

}

fn create_mousekey(name: &str, k_keys: &AttributeSetRef<KeyCode>, m_keys: &AttributeSetRef<KeyCode>, m_axes: &AttributeSetRef<RelativeAxisCode>) -> std::io::Result<VirtualDevice> {

    let mut keys = AttributeSet::new();

    for k in k_keys { //duplication of the keyboard's keycodes
        keys.insert(k) 
    }
    for k in m_keys { //duplication of the mouse's keycodes
        keys.insert(k) 
    }

    let mut rel_axes = AttributeSet::new();
    for r in m_axes {
        rel_axes.insert(r) //duplication of the mouse's keycodes
    }

    let mousekey = VirtualDevice::builder()?
        .name(name)
        .with_relative_axes(&rel_axes)?
        .with_keys(&keys)?
        .build()?;

    Ok(mousekey)
}


fn mouse_arrows(vert: bool, dir: bool, mut axis_lim_time: i32, hold_time: i32) -> (Vec<InputEvent>, i32, KeyCode) {
    let mut rep: KeyCode = KeyCode::KEY_10CHANNELSDOWN; // don't start repeating
    let mut events: Vec<InputEvent> = vec![];

    let mut key: KeyCode;
    // dir: true = right/ down, false = left/ up
    key = match dir {true => {KeyCode::KEY_RIGHT}, false => {KeyCode::KEY_LEFT}};

    if vert == true {
        key = match dir {true => {KeyCode::KEY_DOWN}, false => {KeyCode::KEY_UP}};
    };

    if axis_lim_time == 0 {
        events.push(InputEvent::new(EventType::KEY.0, key.0, 1));
        events.push(InputEvent::new(EventType::KEY.0, key.0, 0));
    }

    if axis_lim_time > hold_time {
        if axis_lim_time < 1000 {
            axis_lim_time = 1000;
            rep = key;
        }
    }

    axis_lim_time += 1;


    return (events, axis_lim_time, rep)
}