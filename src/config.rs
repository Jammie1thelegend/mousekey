
use std::fs;
use serde::Deserialize;

#[derive(Copy, Clone)]
#[derive(Deserialize)]
#[derive(Debug)]
pub struct Config {
    pub move_limit: i32,
    pub hold_time: i32,
    pub show_layer: bool,
}

// improve error handling communication (print warns, errors, etc.)

impl Config {
    pub fn load(path: &str, default_cfg: &str) -> Self {
        match fs::File::open(path) {
            Ok(file) => {file}, 
            Err(_) => {fs::write(path, default_cfg).unwrap(); fs::File::open(path).unwrap()}
        };

        let config_raw = match fs::read_to_string(path) {
            Ok(raw) => {raw}, Err(_) => {fs::write(path, default_cfg).unwrap(); fs::read_to_string(path).unwrap()}
        };

    // add error handling
        let config: Self = toml::from_str(&config_raw).unwrap();
        return config
    }
}