use anyhow::{Error, Result};
use tokio::task::spawn_blocking;
mod client;
mod config;
mod default_services;
mod device;
mod server;
mod service;

use config::{Config, DeviceConfig};
use default_services::{
    mpd::MpdService, page::PageService, sysinfo::SysInfoService, lib::LibService,
    DefaultService,
};
use device::DeviceHandler;
use server::Server;
use tokio::join;

use crate::default_services::{obs::ObsService, pulse::PulseAudioService};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");
    let config = Config {
        device_config: DeviceConfig::default(),
    };
    let server = Server::new(config.device_config.local_port).await?;
    let server_handler = tokio::spawn(async move {
        loop {
            let _ = server.handle().await;
        }
    });
    run_default_service().await?;
    DeviceHandler::new(config.device_config.clone()).await?;
    server_handler.await?;
    Ok(())
}

async fn run_default_service() -> Result<()> {
    let _ = join!(
        LibService::run(),
        PageService::run(),
        SysInfoService::run(),
        MpdService::run(),
        ObsService::run(),
        PulseAudioService::run()
    );
    Ok(())
}
