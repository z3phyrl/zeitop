use super::DefaultService;
use crate::config::Config;
use crate::service::{Reply::Text, Request, RequestService};
use anyhow::{Error, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use sass_rs::{compile_string, Options};
use serde::Serialize;
use serde_json::to_string;
use tokio::{
    fs::{read_to_string, File},
    io::AsyncReadExt,
    task::spawn_blocking,
};

#[derive(Serialize, Clone)]
struct Page {
    name: String,
    content: Option<String>,
    style: Option<String>,
    script: Option<String>,
}

struct PageServiceConfig {
    default_page: String,
}

pub struct PageService {}

impl DefaultService for PageService {
    async fn run() -> Result<()> {
        let mut request = RequestService::new("page").await?;
        tokio::spawn(async move {
            loop {
                if let Some(req) = request.next().await {
                    if let Some((page, path)) = req.request.split_once("/") {
                        let page_dir = Config::dir().join(format!("pages/{page}/"));
                        if !page_dir.exists() {
                            let _ = req.reply(Text("!Invalid Page".to_string())).await;
                            continue;
                        }
                        let asset_path = page_dir.join(path);
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
                    } else {
                        match Page::load(&req.request).await {
                            Ok(page) => {
                                let _ = req.reply(Text(to_string(&page).unwrap_or_default())).await;
                            }
                            Err(e) => {
                                let _ = req.reply(Text(format!("!{e}")));
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }
}

impl Page {
    async fn load(name: &str) -> Result<Self> {
        let page_dir = Config::dir().join(format!("pages/{name}/"));
        if !page_dir.exists() {
            return Err(Error::msg("Invalid Page"));
        }
        let content = read_to_string(page_dir.join("page.html")).await.ok();
        let script = read_to_string(page_dir.join("init.js")).await.ok();
        let mut style = None;
        if let Some(scss) = read_to_string(page_dir.join("style.scss")).await.ok() {
            style = Some(
                spawn_blocking(move || compile_string(&scss, Options::default()))
                    .await?
                    .unwrap_or_default(),
            );
        }
        Ok(Self {
            name: String::from(name),
            content,
            style,
            script,
        })
    }
}
