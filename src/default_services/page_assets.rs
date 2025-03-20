use super::DefaultService;
use crate::config::Config;
use crate::service::{Reply::Text, Request, RequestService};
use anyhow::{Error, Result};
use tokio::{ spawn, fs::File, io::AsyncReadExt };
use std::io::Read;
use base64::{ engine::general_purpose::STANDARD, Engine as _ };

pub struct PageAssetsService;

impl DefaultService for PageAssetsService {
    async fn run() -> Result<()> {
        let mut page_assets = RequestService::new("page-assets").await?;
        spawn(async move {
            loop {
                if let Some(req) = page_assets.next().await {
                    println!("{:?}", req.request);
                    let Some((page, path)) = req.request.split_once("/") else {
                        let _ = req.reply(Text("!Invalid Request".to_string())).await;
                        continue;
                    };
                    let page_dir = Config::dir().join(format!("pages/{page}/"));
                    if !page_dir.exists() {
                        let _ = req.reply(Text("!Invalid Page".to_string())).await;
                        continue;
                    }
                    let asset_path = page_dir.join(format!("assets/{path}"));
                    println!("{page_dir}");
                    if !asset_path.exists() {
                        let _ = req.reply(Text("!Invalid Path".to_string())).await;
                        continue;
                    }
                    let Ok(mut asset_file) = File::open(&asset_path).await else {
                        let _ = req.reply(Text(format!("!Cannot Open {asset_path}"))).await;
                        continue;
                    };
                    let mut asset = Vec::new();
                    let Ok(_) = asset_file.read_to_end(&mut asset).await else {
                        let _ = req.reply(Text(format!("!Cannot Read {asset_path}"))).await;
                        continue;
                    };
                    let _ = req.reply(Text(STANDARD.encode(asset))).await;
                }
            }
        });
        Ok(())
    }
}
