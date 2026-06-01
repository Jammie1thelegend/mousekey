use std::fs;



use toml;
use toml::de::Error;

pub struct Config {
  pub move_limit: i32,
  pub hold_time: i32
}





impl Config {
  pub fn default() -> Self {
    Self {
      move_limit: 40,
      hold_time: 45
    }
  }

  pub fn exists() -> bool {
    match dirs::config_dir() {
      Some(_) => {
        return std::path::Path::new(&Config::path()).exists()
      }
      std::prelude::v1::None => {
        return false
      }
    }
  }

  pub fn path() -> String {

    
    // If running as sudo, construct path using the real user's home
    if user != "root" {
        format!("/home/{}/.config/mouse2joy/config.toml", user)
    } else {
        // Fallback to root's config if not running through sudo
        dirs::config_dir()
            .unwrap_or_default()
            .join("mouse2joy")
            .join("config.toml")
            .to_string_lossy()
            .into_owned()
    }
  }

  // WARN: Can make program crash
  pub fn load() -> Result<Config,Error> {
    let file = Config::path();
    let contents = std::fs::read_to_string(file).unwrap();
    toml::from_str(&contents)
  }

  pub fn move_limit(&self) -> i32 {
    self.move_limit
  }

  pub fn hold_time(&self) -> i32 {
    self.hold_time
  }

}
