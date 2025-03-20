use anyhow::{Error, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use os_path::OsPath;
use std::path::Path;
use toml::from_str;

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceConfig {
    pub usb_ports: Option<Vec<u8>>,
    pub local_port: u16,
    pub remote_port: u16,
    pub app_path: OsPath,
    pub cleaner_path: OsPath,
}

#[cfg(target_os = "linux")]
impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            usb_ports: None,
            local_port: 6969,
            remote_port: 6969,
            app_path: OsPath::from("/usr/share/zeitop/base.apk"),
            cleaner_path: OsPath::from("/usr/share/zeitop/cleaner.jar"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub device_config: DeviceConfig,
}

impl Config {
    pub fn dir() -> OsPath {
        if let Some(proj_dir) = ProjectDirs::from("com", "z3phyrl", "zeitop") {
            proj_dir.config_dir().to_path_buf().into()
        } else {
            panic!("something is broken. fix it.");
        }
    }
}
