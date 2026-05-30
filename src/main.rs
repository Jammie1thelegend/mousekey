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
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/*     let mousekey_devnode = mousekey.enumerate_dev_nodes_blocking().unwrap().last().unwrap();
    let mousekey_path: PathBuf = mousekey_devnode.unwrap();
    let mousekey_path2 = mousekey_path.clone();

    thread::spawn(move || {
        let mut temp_d = Device::open(mousekey_path).unwrap();
        loop{for ev in temp_d.fetch_events().unwrap(){println!("{:?}", ev);}}
    });
    thread::spawn(move || {
        let mut temp_d = Device::open(mousekey_path2).unwrap();
        loop{for ev in temp_d.fetch_events().unwrap(){println!("{:?}", ev);}}
    }); */


    // thread send events to main loop
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
    let (retx, rerx) = mpsc::channel(); // send keycode to arrow repeat thread
    let stop_rep = Arc::new(AtomicUsize::new(0));
    let stop_rep_ar = Arc::clone(&stop_rep);
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
                let mut events: Vec<InputEvent> = vec![ev];
                let rep: KeyCode;
                (events, mousearrow, rep) = mouse_arrows(ev, mousearrow);

                if rep != KeyCode::KEY_10CHANNELSDOWN {
                    if rep == KeyCode::KEY_10CHANNELSUP{
                        stop_rep.store(1, Ordering::Relaxed);
                    }
                    else {
                        stop_rep.store(0, Ordering::Relaxed);
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

                if stop_rep_ar.load(Ordering::Relaxed) == 1 {stop_rep_ar.store(0, Ordering::Relaxed); key = KeyCode::KEY_10CHANNELSUP; break}

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






fn select_input_device(filter_evtype: EventType, filter_rel: RelativeAxisCode, filter_keys: Vec<KeyCode>, devname:&str)-> Result<Device, Mouse2JoyError> {
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

    let mousekey = VirtualDevice::builder()?
        .name(name)
        .with_relative_axes(&rel_axes)?
        .with_keys(&keys)?
        .build()?;

    Ok(mousekey)
}






fn mouse_arrows(input: InputEvent, mut mousearrow: MouseArrow) -> (Vec<InputEvent>, MouseArrow, KeyCode) {
    let move_limit: i32 = 70;
    let hold_time: i32 = 50;

    let mut rep: KeyCode = KeyCode::KEY_10CHANNELSDOWN; // don't start repeating
    let mut events: Vec<InputEvent> = vec![];
    match input.destructure() {
        EventSummary::RelativeAxis(_, RelativeAxisCode::REL_X, _) => {
            mousearrow.x += input.value();

            if mousearrow.x.abs() > move_limit / 2 {mousearrow.y = 0}

            if mousearrow.x.abs() > move_limit {
                let mut dir = 1; //defaults to right, positive movement and KEY_RIGHT
                if input.value() <= 0 {dir = -1};

                let key = match dir {1 => {KeyCode::KEY_RIGHT}, -1 => {KeyCode::KEY_LEFT}, _ => {KeyCode::KEY_RIGHT}};

                mousearrow.x = move_limit*dir;

                if mousearrow.x_lim_time == 0 {
                    events.push(InputEvent::new(EventType::KEY.0, key.0, 1));
                    events.push(InputEvent::new(EventType::KEY.0, key.0, 0));
                }
                
                if mousearrow.x_lim_time > hold_time {
                    if mousearrow.x_lim_time < 1000 {
                        mousearrow.x_lim_time = 1000;
                        rep = key;
                    }
                }

                mousearrow.x_lim_time += 1;
            }

            else {
                if mousearrow.x_lim_time >= hold_time {
                    rep = KeyCode::KEY_10CHANNELSUP
                }

                mousearrow.x_lim_time = 0; 
            }

        },
        EventSummary::RelativeAxis(_, RelativeAxisCode::REL_Y, _) => {
            mousearrow.y += input.value();

            if mousearrow.y.abs() > move_limit / 2 {mousearrow.x = 0}

            if mousearrow.y.abs() > move_limit {
                let mut dir = 1; //defaults to down, positive movement and KEY_DOWN
                if input.value() <= 0 {dir = -1};

                let key = match dir {1 => {KeyCode::KEY_DOWN}, -1 => {KeyCode::KEY_UP}, _ => {KeyCode::KEY_DOWN}};

                mousearrow.y = move_limit*dir;

                if mousearrow.y_lim_time == 0 {
                    events.push(InputEvent::new(EventType::KEY.0, key.0, 1));
                    events.push(InputEvent::new(EventType::KEY.0, key.0, 0));
                }
                
                if mousearrow.y_lim_time > hold_time {
                    if mousearrow.y_lim_time < 1000 {
                        mousearrow.y_lim_time = 1000;
                        rep = key;
                    }
                }

                mousearrow.y_lim_time += 1;
            }

            else {
                if mousearrow.y_lim_time >= hold_time {
                    rep = KeyCode::KEY_10CHANNELSUP
                }

                mousearrow.y_lim_time = 0; 
            }
        },
        _ => {events.push(input)}
    }
    return (events, mousearrow, rep)
}