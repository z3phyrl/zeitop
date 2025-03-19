use anyhow::{Result, Error};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::path::{ PathBuf , Path};
use toml::from_str;

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceConfig {
    pub usb_ports: Option<Vec<u8>>,
    pub local_port: u16,
    pub remote_port: u16,
    pub app_path: PathBuf,
    pub cleaner_path: PathBuf,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            usb_ports: None,
            local_port: 6969,
            remote_port: 6969,
            app_path: PathBuf::from("/usr/share/zeitop/base.apk"),
            cleaner_path: PathBuf::from("/usr/share/zeitop/cleaner.jar"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub device_config: DeviceConfig,
}

impl Config {
    pub fn default_dir() -> Result<PathBuf> {
        if let Some(env) = env::var_os("XDG_CONFIG_HOME") {
            println!("{env:?}");
            Ok(Path::new(&env).join("zeitop/"))
        } else {
            if let Some(env) = env::var_os("HOME") {
                Ok(Path::new(&env).join(".config/zeitop/"))
            } else {
                return Err(Error::msg(
                    "Environment variables unset: $XDG_CONFIG_HOME, $HOME",
                ));
            }
        }
    }
}
