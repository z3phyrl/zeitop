use crate::config::DeviceConfig;
use tokio::task::spawn_blocking;
// use adb::DeviceInfo;
use anyhow::{Error, Result};
// use mozdevice as adb;
use rusb::{
    has_hotplug, ConfigDescriptor, Context, DeviceDescriptor, DeviceHandle, DeviceList,
    GlobalContext, Hotplug, HotplugBuilder, InterfaceDescriptor, Language, Registration,
    UsbContext,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use usb_ids::{FromId, Vendor};

struct HotPlugHandler {
    config: DeviceConfig,
    device_map: DeviceMap,
}

trait DeviceExt {
    fn get_interfaces_name(
        &self,
        lang: Language,
        config_desc: &DeviceDescriptor,
        timeout: Duration,
    ) -> Result<Vec<String>>;
}

impl DeviceExt for DeviceHandle<GlobalContext> {
    fn get_interfaces_name(
        &self,
        lang: Language,
        dev_desc: &DeviceDescriptor,
        timeout: Duration,
    ) -> Result<Vec<String>> {
        let mut interfaces: Vec<String> = vec![];
        for n in 0..dev_desc.num_configurations() {
            if let Ok(conf_desc) = self.device().config_descriptor(n) {
                conf_desc.interfaces().for_each(|i| {
                    i.descriptors().for_each(|d| {
                        interfaces.push(
                            self.read_interface_string(lang, &d, timeout)
                                .unwrap_or_default(),
                        )
                    })
                });
            }
        }
        Ok(interfaces)
    }
}

pub type Serial = String;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Device {
    serial: Serial,
    port_number: u8,
}

impl HotPlugHandler {
    fn handle_device<T: UsbContext>(&self, device: rusb::Device<T>) -> Option<Device> {
        let timeout = Duration::from_secs(1);
        let port = device.port_number();
        let devices = DeviceList::new().expect("Cannot get device list");
        for device in devices.iter() {
            if device.port_number() == port {
                let port_number = device.port_number();
                let dev_desc = device
                    .device_descriptor()
                    .expect("Cannot Get device descriptors");
                if let Ok(handle) = device.open() {
                    let langs = handle
                        .read_languages(timeout)
                        .expect("Cannot Read Languages");
                    let interfaces = handle
                        .get_interfaces_name(langs[0], &dev_desc, timeout)
                        .unwrap();
                    if interfaces.contains(&String::from("ADB Interface"))
                        && (self
                            .config
                            .usb_ports
                            .as_ref()
                            .is_some_and(|p| p.contains(&port_number))
                            || self.config.usb_ports.as_ref().is_none())
                    {
                        let serial = handle
                            .read_serial_number_string(langs[0], &dev_desc, timeout)
                            .unwrap_or(String::new());
                        return Some(Device {
                            serial,
                            port_number,
                        });
                    }
                }
            }
        }
        return None;
    }
}

fn reverse(serial: Serial, remote_port: u16, local_port: u16) {
    if Command::new("adb")
        .args(["-s", &serial, "wait-for-usb-device"])
        .output()
        .is_ok()
    {
        print!(
            " => Reverse :: {}",
            if Command::new("adb")
                .args([
                    "-s",
                    &serial,
                    "reverse",
                    &format!("tcp:{}", remote_port,),
                    &format!("tcp:{}", local_port),
                ])
                .output()
                .is_ok()
            {
                "Ok"
            } else {
                "Error"
            }
        );
    } else {
        print!(" => Device :: Err(\"Do you have ADB installed?\")");
    }
}

fn install_app(app_path: &PathBuf) -> Child {
    Command::new("adb")
        .args(["install", app_path.to_str().unwrap()])
        .spawn()
        .unwrap()
}

fn install_cleaner_script(path: &PathBuf) {
    if Command::new("adb")
        .args(["push", path.to_str().unwrap(), "/data/local/tmp/"])
        .output()
        .is_ok()
    {
        println!("Device => CleanUpScript :: Push");
    }
}

fn run_cleaner_script() {
    if Command::new("adb")
        .args([
            "shell",
            "CLASSPATH=/data/local/tmp/cleaner.jar",
            "nohup",
            "app_process",
            "/",
            "com.z3phyrl.MainKt",
        ])
        .spawn()
        .is_ok()
    {
        println!("Device => CleanUpScript :: Spawned");
    }
}

fn start_app() {
    if Command::new("adb")
        .args([
            "shell",
            "am",
            "start",
            "-n",
            "com.z3phyrl.zeitop/com.z3phyrl.zeitop.MainActivity",
        ])
        .output()
        .is_ok()
    {
        println!("Device => App :: Launched");
    }
}

impl<T: UsbContext> Hotplug<T> for HotPlugHandler {
    fn device_arrived(&mut self, device: rusb::Device<T>) {
        if let Some(dev) = self.handle_device(device) {
            print!("> Port {:?}", dev.port_number);
            if dev.serial.is_empty() {
                print!(" => Device :: Err(\"Your Android device is broken :: No Serial number\")");
                return;
            }
            self.device_map
                .lock()
                .unwrap()
                .insert(dev.serial.clone(), dev.clone());
            print!(" => Serial :: {}", dev.serial);
            reverse(dev.serial, self.config.remote_port, self.config.local_port);
            let mut app_install = install_app(&self.config.app_path);
            println!();
            install_cleaner_script(&self.config.cleaner_path);
            if app_install.wait().is_ok() {
                println!("Device => App :: Installed");
            }
            run_cleaner_script();
            start_app();
        }
    }
    fn device_left(&mut self, device: rusb::Device<T>) {
        println!("< Port {:?}", device.port_number());
        self.device_map
            .lock()
            .unwrap()
            .retain(|_k, v| !(v.port_number == device.port_number()));
    }
}

pub struct DeviceHandler {
    context: Context,
    _reg: Option<Registration<Context>>,
    pub device_map: DeviceMap,
}

pub type DeviceMap = Arc<Mutex<HashMap<Serial, Device>>>;

impl DeviceHandler {
    pub fn new(config: DeviceConfig) -> Result<Self> {
        if has_hotplug() && Command::new("adb").arg("start-server").output().is_ok() {
            let device_map = Arc::new(Mutex::new(HashMap::new()));
            let context = Context::new()?;
            let _reg: Option<Registration<Context>> =
                Some(HotplugBuilder::new().enumerate(true).register(
                    &context,
                    Box::new(HotPlugHandler {
                        config,
                        device_map: device_map.clone(),
                    }),
                )?);
            device_map.lock().unwrap().insert(
                String::from(String::from("unknown")),
                Device {
                    serial: String::from("unknown"),
                    port_number: 0,
                },
            );
            Ok(Self {
                context,
                _reg,
                device_map,
            })
        } else {
            Err(Error::msg("hotplug api is not supported"))
        }
    }
    pub fn handle(&self) {
        self.context.handle_events(None).unwrap();
    }
}
