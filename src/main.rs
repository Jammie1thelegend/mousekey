use evdev::{EventSummary};
use evdev::{
    AttributeSet, AttributeSetRef, Device, EventType, InputEvent, KeyCode, RelativeAxisCode, uinput::VirtualDevice
};
use std::fs;
use thiserror::Error;
use log::{info, warn, error, LevelFilter};
use env_logger::Builder;

use std::{thread, time};
use std::sync::mpsc;
use std::os::unix::thread::JoinHandleExt;
use libc::pthread_cancel;

//windows
/* use std::os::windows::thread::JoinHandleExt;
use kernel32::TerminateThread; */

const VDEVICE_NAME: &str = "mousekey";


#[derive(Error, Debug)]
pub enum Mouse2JoyError {
    #[error("Failed to find a compatible input device. Make sure you are running the application with root priviledges.")]
    NoDeviceError,

    #[error("Failed to read a mouse input")]
    FailedToReadInput,
}

struct MouseArrow {
    x: i32,
    y: i32,
    x_lim_time: i32,
    y_lim_time: i32,
}

impl Copy for MouseArrow {}

impl Clone for MouseArrow {
    fn clone(&self) -> Self {
        *self
    }
}

fn main() -> Result<(), Mouse2JoyError> {

    // initialize logger
    Builder::new()
        .filter_level(LevelFilter::Trace)  // This shows everything
        .init();
    

    // select mouse device
    let mut mouse = select_input_device(
        EventType::RELATIVE, 
        RelativeAxisCode::REL_X, 
        KeyCode::BTN_LEFT, 
        "mouse"
    ).unwrap();

    //select keyboard device
    let mut keyboard = select_input_device(
        EventType::KEY, 
        RelativeAxisCode::REL_MISC, 
        KeyCode::KEY_W, 
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


    // thread external transmission
    let (ktx, rx) = mpsc::channel(); // for keyboard thread
    let mtx = ktx.clone(); // for mouse thread
    let rtx = ktx.clone(); // for arrow key repeat thread

    //keyboard thread
    thread::spawn(move || {
        let _ = keyboard.grab();
        let mut quit_combo = 0;
        loop{
            for ev in keyboard.fetch_events().unwrap(){
                let ev_code: u16 = ev.code();
                if ev_code == KeyCode::KEY_F5.0 || ev_code ==  KeyCode::KEY_F7.0 || ev_code ==  KeyCode::KEY_F8.0 {
                    quit_combo += match ev.value() {0 => {-1}, 1 => {1}, _ => {0}};
                    if quit_combo == 3 {panic!("__ ___ FORCE QUIT ___ __");}
                }

                ktx.send(ev).unwrap()
            }
        }
    });

    //mouse thread
    let (artx, rrx) = mpsc::channel(); // sender for mouse_arrows func / thread
    thread::spawn(move || {

        let mut mousearrow = MouseArrow {
        x: 0,
        y: 0,
        x_lim_time: 0,
        y_lim_time: 0
        };

        let _ = mouse.grab();

        loop{
            for ev in mouse.fetch_events().unwrap(){
                let rep: (bool, u16);
                let mut events: Vec<InputEvent> = vec![ev];
                (events, mousearrow, rep) = mouse_arrows(ev, mousearrow);
                if rep != (false, KeyCode::KEY_10CHANNELSDOWN.0) { // if not empty
                    let _ = artx.send(rep);
                }
                for e in events {mtx.send(e).unwrap()}
            }
        }
    });

    thread::spawn(move || { // arrow key repeat thread
        loop{
            let rep = rrx.recv().unwrap();
            while rrx.recv().unwrap() != (false, KeyCode::KEY_LEFT.0) {}
            thread::sleep(time::Duration::from_millis(50));
            let _ = rtx.send(InputEvent::new(EventType::KEY.0, rep.1, 1));
            let _ = rtx.send(InputEvent::new(EventType::KEY.0, rep.1, 0)); 
        }
    });

    // re-emits events through unified input device, checks for force quit and converts mouse into arrow keys
    loop {
        let input: InputEvent = rx.recv().unwrap();

        //let mut events: Vec<InputEvent> = vec![];
        let _ = &mousekey.emit(&[input]);


        //println!("{:?}", input);
         }
    }






fn select_input_device(filter_evtype: EventType, filter_rel: RelativeAxisCode, filter_key: KeyCode, devname:&str)-> Result<Device, Mouse2JoyError> {
    // find all input devices that can be used as a specific type of device

    let dev_input_paths: Vec<_> = fs::read_dir("/dev/input/by-id")
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.path().into_os_string().to_str().map(String::from)).collect();

    let mut devices: Vec<Device> = Vec::new();
    for p in dev_input_paths {
        let d = Device::open(&p).ok().filter(|device| device.supported_events().contains(filter_evtype));
        match d {
            Some(d) => {
                if filter_rel != RelativeAxisCode::REL_MISC {
                    if d.supported_relative_axes().map_or(false, |keys| keys.contains(filter_rel)) == false {continue}
                }
                if d.supported_keys().map_or(false, |keys| keys.contains(filter_key)) == false {continue}
                devices.push(d); 
                },
            _ => continue
        }
    }
    
    if devices.is_empty() {
        error!("{}", Mouse2JoyError::NoDeviceError);
        return Err(Mouse2JoyError::NoDeviceError);
    }

    // ask user which device to use

    let mut index: usize = 1;
    if !(devices.len() == 1) {
        println!("Several {}s detected, please select one:", devname);
        for (i, device) in devices.iter().enumerate() {
            println!("{}: {}, more info: {:?}", i + 1, device.name().unwrap_or("Unknown Device"), device.input_id());
        };
        index = input_in_range(1, devices.len())
    } else {
        warn!("Only one compatible {} found!", devname);
    }
    
    let input_device = devices.remove(index - 1);
    info!("Using \"{}\" as {} input device", input_device.name().unwrap_or("Unknown Device"), devname);
    Ok(input_device)

}

// ask user for a usize input within a given range
fn input_in_range(min: usize, max: usize) -> usize {
    let mut input = String::new();

    loop {
        input.clear();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        match input.trim().parse::<usize>() {
            Ok(index) if index >= min && index <= max => {
                return index;
            }
            _ => {
                println!(
                    "Invalid selection. Please enter a number between {} and {}",
                    min, max
                );
                continue;
            }
        }
    }
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

    let joystick = VirtualDevice::builder()?
        .name(name)
        .with_relative_axes(&rel_axes)?
        .with_keys(&keys)?
        .build()?;

    Ok(joystick)
}

//                                                                  events,         mousearrow,  arrow repeat
fn mouse_arrows(input: InputEvent, mut mousearrow: MouseArrow) -> (Vec<InputEvent>, MouseArrow, (bool, u16)) {
    let move_limit: i32 = 70;
    let hold_time: i32 = 50;

    let mut rep: (bool, u16) = (false, KeyCode::KEY_10CHANNELSDOWN.0); // essentially empty

    let mut events: Vec<InputEvent> = vec![];
    match input.destructure() {
        EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X, _) => {
            mousearrow.x += input.value();

            if mousearrow.x > move_limit {
                mousearrow.x = move_limit;
                if mousearrow.x_lim_time == 0 {
                    events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_RIGHT.0, 1));
                    events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_RIGHT.0, 0));
                }
                if mousearrow.x_lim_time >= hold_time {
                    rep = (true, KeyCode::KEY_RIGHT.0);
                } 
                mousearrow.x_lim_time += 1;
            }
            else if mousearrow.x < -move_limit {
                mousearrow.x = -move_limit;
                if mousearrow.x_lim_time == 0 {
                    events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_LEFT.0, 1));
                    events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_LEFT.0, 0));
                }
                if mousearrow.x_lim_time >= hold_time {
                    rep = (true, KeyCode::KEY_LEFT.0);
                } 
                mousearrow.x_lim_time += 1;
            }
            else {
                events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_RIGHT.0, 0));
                events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_LEFT.0, 0));
                if mousearrow.x_lim_time >= hold_time {
                    rep = (false, KeyCode::KEY_LEFT.0);
                }
                mousearrow.x_lim_time = 0;
            }

        },
        EventSummary::RelativeAxis(_, RelativeAxisCode::REL_Y, _) => {
            /* mousearrow.y += input.value();
            if mousearrow.y.abs() > 1 {
                if mousearrow.y > 0 {events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_UP.0, 1)); 
                events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_UP.0, 0))}
                
                else {events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_DOWN.0, 1)); 
                events.push(InputEvent::new(EventType::KEY.0, KeyCode::KEY_DOWN.0, 0))}
            } */
        },
        _ => {events.push(input)}
    }
    //println!("x {}, x_lim_time {}", mousearrow.x, mousearrow.x_lim_time);
    return (events, mousearrow, rep)
}