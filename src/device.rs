use crate::config::DeviceConfig;
use anyhow::Result;
use futures::stream::{Stream, StreamExt};
use nusb::{hotplug::HotplugEvent, watch_devices, DeviceId, DeviceInfo};
use std::path::PathBuf;
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

async fn install(serial: &str, path: PathBuf) -> Result<()> {
    adb(serial, ["install", path.to_str().unwrap_or_default()]).await?;
    Ok(())
}

async fn push_cleaner(serial: &str, path: PathBuf) -> Result<()> {
    adb(
        serial,
        [
            "push",
            path.to_str().unwrap_or_default(),
            "/data/local/tmp/",
        ],
    )
    .await?;
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
        let mut watch = watch_devices()?;
        spawn(async move {
            loop {
                match watch.next().await {
                    Some(HotplugEvent::Connected(info)) => {
                        let Some(serial) = info.serial_number() else {
                            return;
                        };
                        wait_for(serial).await.unwrap();
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
                        start_app(serial).await.unwrap();
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
}
