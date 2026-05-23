use evdev::{EventSummary, SynchronizationCode};
use evdev::{
    AttributeSet, AttributeSetRef, Device, EventType, InputEvent, KeyCode, RelativeAxisCode, uinput::VirtualDevice
};
use std::fs;
use thiserror::Error;
use log::{info, warn, error, LevelFilter};
use env_logger::Builder;

use std::thread;
use std::sync::mpsc;

use std::sync::atomic::{AtomicUsize, Ordering};

static QUIT_COMBO: AtomicUsize = AtomicUsize::new(0);

const VDEVICE_NAME: &str = "mousekey";


#[derive(Error, Debug)]
pub enum Mouse2JoyError {
    #[error("Failed to find a compatible input device. Make sure you are running the application with root priviledges.")]
    NoDeviceError,

    #[error("Failed to read a mouse input")]
    FailedToReadInput,
}

struct MouseArrow {
    left: i32,
    right: i32,
    up: i32,
    down: i32
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
    

    // set up merged uinput device
    let mut mousekey = create_mousekey(
        VDEVICE_NAME, 
        keyboard.supported_keys().unwrap(), 
        mouse.supported_keys().unwrap(), 
        mouse.supported_relative_axes().unwrap()
    ).unwrap();


    // thread external transmission
    let (ktx, rx) = mpsc::channel();
    let mtx = ktx.clone();

    //keyboard thread
    thread::spawn(move || {
        let _ = keyboard.grab();
        loop{
            for ev in keyboard.fetch_events().unwrap(){
                ktx.send(ev).unwrap()
            }
        }
    });

    //mouse thread
    thread::spawn(move || {
        let _ = mouse.grab();
        loop{
            for ev in mouse.fetch_events().unwrap(){
                mtx.send(ev).unwrap()
            }
        }
    });

    let mut mousearrow = MouseArrow {
        left: 0,
        right: 0,
        up: 0,
        down: 0
    };


    // re-emits events through unified input device
    loop {
        let input: InputEvent = rx.recv().unwrap();
        let _ = mousekey.emit(&[mouse_to_button(input, &mousearrow)]); // processes input, and changes it if neccessary (eg mouse move to arrow key)


        //println!("{:?}", input.value());
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

    println!("{:?}", keys);

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

fn mouse_to_button(input: InputEvent, mousearrow: &MouseArrow) -> InputEvent {
    let empty_ev = InputEvent::new(EventType::SYNCHRONIZATION.0, SynchronizationCode::SYN_REPORT.0, 1);

    let mut b_type: EventType = input.event_type();
    let mut b_code: u16 = input.code();
    let mut b_value: i32 = input.value();

    if !(b_type == EventType::KEY || b_type == EventType::RELATIVE) {
        let button:InputEvent = input;
        return button
    }

    match input.destructure() {
        // checks for force quit combo (F5+F7+F8) **NB only works if correct keyboard selected, must make sure keyboard works before activation of program!!
        EventSummary::Key(_, KeyCode::KEY_F5 | KeyCode::KEY_F7 | KeyCode::KEY_F8, _) => {
            match input.value() {
                0 => {QUIT_COMBO.fetch_sub(1, Ordering::SeqCst);}
                1 => {QUIT_COMBO.fetch_add(1, Ordering::SeqCst);}
                _ => {}
            }
            if QUIT_COMBO.load(Ordering::SeqCst) == 3 {panic!("__ ___ FORCE QUIT ___ __");}
        }
        

        EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X, _) => {

            if input.value() < 0 {b_type = EventType::KEY; b_code = KeyCode::KEY_LEFT.0; b_value = 1}
            else {b_type = EventType::KEY; b_code = KeyCode::KEY_RIGHT.0; b_value = 1}

        }
        _ => {}
    }
    let button:InputEvent = InputEvent::new(b_type.0, b_code, b_value);
    println!("{:?}", button);
    return button
}