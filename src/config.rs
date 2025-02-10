use crate::ao3::common::DownloadFormat;

use directories::ProjectDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{create_dir, File};
use std::io::Read;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub port: u16,
    pub download_path: String,
    pub ao3_username: Option<String>,
    pub ao3_password: Option<String>,
    pub default_format: DownloadFormat,
    pub devices: Vec<Device>,
    pub fandom_map: HashMap<String, String>,
    pub fandom_filter: HashMap<String, Vec<String>>,
}

impl Config {
    pub fn get_device_by_name(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }
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

pub async fn read_config() -> Result<Config, String> {
    if let Some(proj_dirs) = ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
        let config_dir = proj_dirs.config_dir();
        if config_dir.exists() {
            let mut file = match File::open(config_dir.join("config.toml")) {
                Ok(file) => file,
                Err(_) => return Err(format!(
                    "Failed to open config.toml at {}, make sure the file exists and has the right permissions",
                    config_dir.display()
                )),
            };
            let mut file_contents = String::new();
            let read_result = file.read_to_string(&mut file_contents);
            if read_result.is_err() {
                return Err(read_result.err().unwrap().to_string());
            };

            match toml::from_str::<Config>(&file_contents) {
                Ok(config) => Ok(config),
                Err(error) => Err(error.to_string()),
            }
        } else {
            create_dir(proj_dirs.config_dir()).unwrap();
            Err(format!(
                "First time run, create a config file at {}",
                config_dir.join("config.toml").display()
            ))
        }
    } else {
        Err(String::from(
            "Failed to get home directory from OS, make sure home path is set correctly in OS",
        ))
    }
}
