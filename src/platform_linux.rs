use evdev::{
    AttributeSet, uinput::VirtualDevice, Device, EventType, InputEvent, KeyCode, RelativeAxisCode,
};
use std::fs;
use log::{info, warn, error, LevelFilter};
use env_logger::Builder;

use std::{thread, time};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;

use std::process::Command;


use crate::mousekey::{Config, MouseArrow, MouseKeyError, QUIT_KEYS};


static PASSTHROUGH: AtomicBool = AtomicBool::new(false);
static STOP_REP: AtomicBool = AtomicBool::new(false);


pub fn main() -> Result<(), MouseKeyError> {

    let kanata_config_path = "mousekey.kbd";
    let mut emitter_keys = AttributeSet::new();

    emitter_keys.insert(KeyCode::KEY_F14);
    emitter_keys.insert(KeyCode::KEY_UP);
    emitter_keys.insert(KeyCode::KEY_DOWN);
    emitter_keys.insert(KeyCode::KEY_LEFT);
    emitter_keys.insert(KeyCode::KEY_RIGHT);

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

    //check kanata_config_path is a valid file

    match fs::File::open(kanata_config_path) {
        Ok(file) => {file}, 
        Err(_) => {panic!("could not find/ open mousekey.kbd!")}
    };


    // make sure log-layer-changes is no when starting program (--log-layer-changes will not work in this case)

    let kanata_config_str = match fs::read_to_string(kanata_config_path) {
        Ok(raw) => {raw}, Err(_) => {panic!("could not read mousekey.kbd!")}
    };
    let kanata_config_str_no_layer = kanata_config_str.replace("log-layer-changes yes", "log-layer-changes no");

    if kanata_config_str_no_layer != kanata_config_str {fs::write(kanata_config_path, kanata_config_str_no_layer).unwrap();}

    //create emitter device -> all virtual devices must be made before kanata is started, so kanata can see them.
    let mut emitter_device = create_emitter_device("mousekey_show_layers_device", &emitter_keys).unwrap();



    // start kanata
    println!("\nStarting Kanata:\n");

    let mut kanata = Command::new("./kanata_linux_x64")
        .arg("--cfg")
        .arg(kanata_config_path)
        .spawn()
        .unwrap();
    

    thread::sleep(time::Duration::from_millis(3000)); // creates a gap between warning message and program begin

    println!("\nKanata started. Continuing...\n");

    // select mouse device
    let mut mouse = select_input_device(
        EventType::RELATIVE, 
        RelativeAxisCode::REL_X, 
        vec![KeyCode::BTN_LEFT], 
        "mouse"
    ).unwrap().0;

    //select keyboard device
    let mut keyboard = select_input_device(
        EventType::KEY, 
        RelativeAxisCode::REL_MISC, 
        vec![KeyCode::KEY_F5, KeyCode::KEY_F7, KeyCode::KEY_F8], 
        "keyboard"
    ).unwrap().0;

    thread::sleep(time::Duration::from_millis(100)); // gives enter key time to return to 0

    //start logging layer changes if config.toml has "show_layer = true"

    if Config::load().show_layer == true {

        let kanata_config_str = match fs::read_to_string(kanata_config_path) {
            Ok(raw) => {raw}, Err(_) => {panic!("could not read mousekey.kbd to enable showing the current layer!")}
        };
        let kanata_config_str_no_layer = kanata_config_str.replace("log-layer-changes no", "log-layer-changes yes");

        if kanata_config_str_no_layer != kanata_config_str {
            fs::write(kanata_config_path, kanata_config_str_no_layer).unwrap();
            let _ = emitter_device.emit(&[
                InputEvent::new(EventType::KEY.0, KeyCode::KEY_F14.0, 1),
                InputEvent::new(EventType::KEY.0, KeyCode::KEY_F14.0, 0)
            ]);

            println!("\nlayers now shown\n");
        }

    }



    // thread send events to main loop
    let (cltx, clrx) = mpsc::channel();

    let mousepassthrough_key = KeyCode::KEY_F13.0;

    //keyboard thread
    thread::spawn(move || {
        let mut quit_combo: i8 = 0;
        
        
        loop{
            for ev in keyboard.fetch_events().unwrap(){
                let ev_code: u16 = ev.code();
                if QUIT_KEYS.contains(&ev_code) {
                    quit_combo += match ev.value() {0 => {-1}, 1 => {1}, _ => {0}};
                    if quit_combo < 0 {quit_combo = 0}
                    if quit_combo == 3 {
                        kanata.kill().expect("WARNING!! Kanata could not be killed!!!");
                        panic!("__ ___ FORCE QUIT ___ __");
                    }
                }

                if ev_code == mousepassthrough_key {let _ = cltx.send(ev.value());}

                //ktx.send(ev).unwrap()
            }
        }
    });

    // capslock/mouse passthrough thread
    thread::spawn(move || {
        loop {
            let val = clrx.recv().unwrap();
            if val == 2 {PASSTHROUGH.store(true, Ordering::Relaxed);}
            else if PASSTHROUGH.load(Ordering::Relaxed) == true {
                PASSTHROUGH.store(false, Ordering::Relaxed);
            }
        }
    });


    
    let (retx, rerx) = mpsc::channel(); // send keycode to arrow repeat thread

    let mut ar_key_device = create_emitter_device("mousekey_emitter", &emitter_keys).unwrap();

    let emitter_dev_path;

    let emitter_paths: Vec<_> = ar_key_device.enumerate_dev_nodes_blocking().unwrap().collect();
    emitter_dev_path = match &emitter_paths[0] {Ok(path) => path, Err(_) => {panic!()}}; 

    thread::spawn(move || { // arrow key emitter thread
        loop{
            let _ = ar_key_device.emit(&[rerx.recv().unwrap()]);
        }
    });

    let mut mousearrow = MouseArrow::load_default();

    let mut main_send_mouse = Device::open(emitter_dev_path).unwrap();

    loop {
        for ev in mouse.fetch_events().unwrap(){
            if ev.event_type() != EventType::RELATIVE {continue}
            if PASSTHROUGH.load(Ordering::Relaxed) == true {
                    STOP_REP.store(true, Ordering::Relaxed);
                    mousearrow = MouseArrow::set_default(&mut mousearrow);
                    continue
                }
            let mut events = vec![];

            if ev.code() == RelativeAxisCode::REL_X.0 {
                mousearrow = process_mouse_m(ev.value(), 0, mousearrow, retx.clone());
                events.push(InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_X.0, -ev.value()));
            }
            
            else if ev.code() == RelativeAxisCode::REL_Y.0 {
                mousearrow = process_mouse_m(0, ev.value(), mousearrow, retx.clone());
                events.push(InputEvent::new(EventType::RELATIVE.0, RelativeAxisCode::REL_Y.0, -ev.value()));
            }

            let _ = main_send_mouse.send_events(&events);
        }
    };
}


fn process_mouse_m(x: i32, y: i32, mut mousearrow: MouseArrow, rettx: Sender<InputEvent>) -> MouseArrow {

    mousearrow.x += x;
    mousearrow.y += y;

    // "normalise" -> we only want one axis at a time to be near move_limit

    if mousearrow.x.abs() > ((mousearrow.move_limit as f32 * 0.75) as i32) {mousearrow.y = 0};
    if mousearrow.y.abs() > ((mousearrow.move_limit as f32 * 0.75) as i32) {mousearrow.x = 0};

    if mousearrow.x.abs() >= mousearrow.move_limit {
        let mut capped_x = -mousearrow.move_limit;

        let mut key = KeyCode::KEY_LEFT;
        if mousearrow.x > 0 {key = KeyCode::KEY_RIGHT; capped_x = mousearrow.move_limit};

        mousearrow.x = capped_x;

        mousearrow.x_lim_time = mouse_arrows(key,mousearrow.x_lim_time, mousearrow.hold_time, rettx);
        
    }

    else if mousearrow.y.abs() >= mousearrow.move_limit {
        let mut capped_y = -mousearrow.move_limit;

        let mut key = KeyCode::KEY_UP;
        if mousearrow.y > 0 {key = KeyCode::KEY_DOWN; capped_y = mousearrow.move_limit};

        mousearrow.y = capped_y;

        mousearrow.y_lim_time = mouse_arrows(key,mousearrow.y_lim_time, mousearrow.hold_time, rettx);
        
    }
    else {
        if STOP_REP.load(Ordering::Relaxed) == false {STOP_REP.store(true, Ordering::Relaxed);}
        mousearrow.x_lim_time = 0;
        mousearrow.y_lim_time = 0;
    }

    return mousearrow

}


fn mouse_arrows(key: KeyCode, mut axis_lim_time: i32, hold_time: i32, rettx: Sender<InputEvent>) -> i32 {

    if axis_lim_time == 0 {
        let _ = rettx.send(InputEvent::new(EventType::KEY.0, key.0, 1));
        let _ = rettx.send(InputEvent::new(EventType::KEY.0, key.0, 0));
    }

    if axis_lim_time > hold_time {
        if axis_lim_time < 1000 {
            axis_lim_time = 1000;
            STOP_REP.store(false, Ordering::Relaxed);
            create_m_ar_thread(key, rettx.clone());
        }
    }

    axis_lim_time += 1;
    return axis_lim_time

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



fn select_input_device(filter_evtype: EventType, filter_rel: RelativeAxisCode, filter_keys: Vec<KeyCode>, devname:&str)-> Result<(Device, String), MouseKeyError> {
    // find all input devices that can be used as a specific type of device

    let dev_input_paths: Vec<_> = fs::read_dir("/dev/input/")
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
    
    
    let input_device = Device::open(&path).unwrap();
    info!("Using \"{}\" as {} input device", input_device.name().unwrap_or("Unknown Device"), devname);
    Ok((input_device, path))

}

fn create_emitter_device(name: &str, keys: &AttributeSet<KeyCode>) -> std::io::Result<VirtualDevice> {

    let emitter_device = VirtualDevice::builder()?
        .name(name)
        .with_keys(&keys)?
        .build()?;

    Ok(emitter_device)
}