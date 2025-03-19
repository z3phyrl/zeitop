use anyhow::{Error, Result};
use tokio::task::spawn_blocking;
mod client;
mod config;
mod default_services;
mod device;
mod server;
mod service;

use config::{Config, DeviceConfig};
use device::DeviceHandler;
use server::Server;
use default_services::DefaultService;
use default_services::page::PageService;
use default_services::sysinfo::SysInfoService; 
use default_services::mpd::MpdService;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");
    let config = Config {
        device_config: DeviceConfig::default(),
    };
    DeviceHandler::new(config.device_config.clone()).await?;
    let server = Server::new(
        config.device_config.local_port,
    )
    .await?;
    let server_handler = tokio::spawn(async move {
        loop {
            let _ = server.handle().await;
        }
    });
    run_default_service().await?;
    server_handler.await?;
    Ok(())
}

async fn run_default_service() -> Result<()> {
    PageService::run().await?;
    SysInfoService::run().await?;
    MpdService::run().await?;
    Ok(())
}
