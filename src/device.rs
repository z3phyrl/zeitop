use crate::config::DeviceConfig;
use anyhow::{Error, Result};
use futures::stream::{Stream, StreamExt};
use nusb::{hotplug::HotplugEvent, list_devices, watch_devices, DeviceId, DeviceInfo};
use os_path::OsPath;
use tokio::process::Command;
use tokio::{join, spawn};

pub type Serial = String;

static PACK_NAME: &str = "com.z3phyrl.zeitop";
static MAIN_CLASS: &str = "com.z3phyrl.MainKt";
static LOCAL_PORT: u16 = 6969;
static REMOTE_PORT: u16 = 6969;

async fn adb<'a, I>(serial: &'a str, args: I) -> Result<std::process::Output, std::io::Error>
where
    I: IntoIterator<Item = &'a str>,
{
    Command::new("adb")
        .args(["-s", serial].into_iter().chain(args))
        .output()
        .await
}

async fn wait_for(serial: &str) -> Result<()> {
    adb(serial, ["wait-for-usb-device"]).await?;
    Ok(())
}

async fn reverse(serial: &str, local: u16, remote: u16) -> Result<()> {
    adb(
        serial,
        ["reverse", &format!("tcp:{remote}"), &format!("tcp:{local}")],
    )
    .await?;
    Ok(())
}

async fn is_installed(serial: &str) -> Result<bool> {
    if adb(serial, ["shell", "pm", "list", "packages", PACK_NAME])
        .await?
        .stdout
        .len()
        > 0
    {
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn install(serial: &str, path: OsPath) -> Result<()> {
    adb(serial, ["install", &path.to_string()]).await?;
    Ok(())
}

async fn push_cleaner(serial: &str, path: OsPath) -> Result<()> {
    adb(serial, ["push", &path.to_string(), "/data/local/tmp/"]).await?;
    Ok(())
}

async fn start_cleaner(serial: &str) -> Result<()> {
    adb(
        serial,
        [
            "shell",
            "CLASSPATH=/data/local/tmp/cleaner.jar",
            "nohup",
            "app_process",
            "/",
            MAIN_CLASS,
            "</dev/null",
            ">/dev/null",
            "2>&1",
        ],
    )
    .await?;
    Ok(())
}

async fn start_app(serial: &str) -> Result<()> {
    let activity = String::from(PACK_NAME) + "/" + PACK_NAME + ".MainActivity";
    adb(serial, ["shell", "am", "start", "-n", &activity]).await?;
    println!("{activity}");
    Ok(())
}

pub struct DeviceHandler {}
impl DeviceHandler {
    pub async fn new(config: DeviceConfig) -> Result<Self> {
        let list = list_devices()?;
        for info in list {
            DeviceHandler::handle_device(config.clone(), info)
                .await
                .unwrap();
        }
        let mut watch = watch_devices()?;
        spawn(async move {
            loop {
                match watch.next().await {
                    Some(HotplugEvent::Connected(info)) => {
                        DeviceHandler::handle_device(config.clone(), info)
                            .await
                            .unwrap();
                    }
                    Some(HotplugEvent::Disconnected(id)) => {
                        println!("< {id:?}");
                    }
                    None => {
                        println!("heh");
                    }
                }
            }
        });
        Ok(Self {})
    }
    async fn handle_device(config: DeviceConfig, info: DeviceInfo) -> Result<()> {
        if !(info
            .interfaces()
            .filter(|i| i.interface_string().is_some_and(|i| i == "ADB Interface"))
            .count()
            > 0)
        {
            return Ok(());
        }
        let Some(serial) = info.serial_number() else {
            return Err(Error::msg("No Serial Number"));
        };
        wait_for(serial).await?;
        let _ = join!(reverse(serial, LOCAL_PORT, REMOTE_PORT), async {
            if !is_installed(serial).await.is_ok_and(|i| i) {
                let _ = join!(
                    install(serial, config.app_path.clone()),
                    push_cleaner(serial, config.cleaner_path.clone()),
                );
                let serial = serial.to_owned();
                spawn(async move {
                    let _ = start_cleaner(&serial).await;
                });
            }
        });
        start_app(serial).await?;
        Ok(())
    }
}
