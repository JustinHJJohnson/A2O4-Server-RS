use crate::ao3::common::DownloadFormat;

use directories::ProjectDirs;
use serde::Deserialize;
use indexmap::IndexMap;
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
    pub fandom_filter: IndexMap<String, Vec<String>>,
}

impl Config {
    pub(crate) fn new() -> Self {
        Self {
            port: Default::default(),
            download_path: Default::default(),
            ao3_username: Default::default(),
            ao3_password: Default::default(),
            default_format: DownloadFormat::EPUB,
            devices: Default::default(),
            fandom_map: Default::default(),
            fandom_filter: Default::default(),
        }
    }
    
    pub fn get_device_by_name(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }
    
    pub fn get_device_by_name_or_first(&self, name: Option<&str>) -> &Device {
        if let Some(device_name) = name {
            match self.get_device_by_name(device_name) {
                Some(device) => device,
                None => self.devices.first().unwrap()
            }
        } else {
            self.devices.first().unwrap()
        }
    }
    
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
    
    pub fn download_path(mut self, download_path: String) -> Self {
        self.download_path = download_path;
        self
    }
    
    pub fn ao3_username(mut self, username: String) -> Self {
        self.ao3_username = Some(username);
        self
    }
    
    pub fn ao3_password(mut self, password: String) -> Self {
        self.ao3_password = Some(password);
        self
    }
    
    pub fn default_format(mut self, format: DownloadFormat) -> Self {
        self.default_format = format;
        self
    }
    
    pub fn devices(mut self, devices: Vec<Device>) -> Self {
        self.devices = devices;
        self
    }
    
    pub fn fandom_map(mut self, fandom_map: HashMap<String, String>) -> Self {
        self.fandom_map = fandom_map;
        self
    }
    
    pub fn fandom_filter(mut self, fandom_filter: IndexMap<String, Vec<String>>) -> Self {
        self.fandom_filter = fandom_filter;
        self
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
            let Ok(mut file) = File::open(config_dir.join("config.toml")) else { 
                return Err(format!(
                    "Failed to open config.toml at {}, make sure the file exists and has the right permissions",
                    config_dir.display()
                ))
            };
            let mut file_contents = String::new();
            let read_result = file.read_to_string(&mut file_contents);
            if read_result.is_err() {
                return Err(read_result.err().unwrap().to_string());
            }

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
