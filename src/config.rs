use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub download_path: String,
    pub ao3_username: Option<String>,
    pub ao3_password: Option<String>,
    pub devices: Vec<Device>,
    pub fandom_map: HashMap<String, String>,
    pub fandom_filter: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Device {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub download_folder: String,
    pub uses_koreader: Option<bool>,
}

pub fn read_config() -> Config {
    let mut file = File::open("config.toml").unwrap();
    let mut file_contents = String::new();
    let _ = file.read_to_string(&mut file_contents); //TODO handle error
    let config: Config = toml::from_str(&file_contents).unwrap();
    return config;
}
