use super::DefaultService;
use crate::config::Config;
use crate::service::{Reply, Reply::Text, Request, RequestService};
use anyhow::{Error, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use os_path::OsPath;
use sass_rs::{Options, OutputStyle, compile_string};
use serde::Serialize;
use serde_json::to_string;
use tokio::{
    fs::{File, read_dir, read_to_string},
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

#[derive(Serialize)]
struct FileEntry {
    name: String,
    file_type: Option<FileType>,
}

#[derive(Serialize)]
enum FileType {
    File,
    Directory,
    Symlink,
}

async fn list_dir(path: &OsPath) -> Result<Vec<FileEntry>> {
    let Ok(mut dir) = read_dir(&path).await else {
        return Err(Error::msg(format!("Error Reading Directory {}", &path)));
    };
    let mut entries: Vec<FileEntry> = Vec::new();
    while let Ok(Some(entry)) = dir.next_entry().await {
        let Ok(file_type) = entry.file_type().await.map(|t| {
            if t.is_file() {
                Some(FileType::File)
            } else if t.is_dir() {
                Some(FileType::Directory)
            } else if t.is_symlink() {
                Some(FileType::Symlink)
            } else {
                None
            }
        }) else {
            continue;
        };
        entries.push(FileEntry {
            name: entry.file_name().into_string().unwrap_or_default(),
            file_type,
        });
    }
    Ok(entries)
}

impl DefaultService for PageService {
    async fn run() -> Result<()> {
        let mut request = RequestService::new("page").await?;
        tokio::spawn(async move {
            loop {
                if let Some(req) = request.next().await {
                    if let Some((page, path)) = req.request.split_once("/") {
                        let page_dir = Config::dir().join(format!("pages/{page}/"));
                        if !page_dir.exists() {
                            let _ = req.reply(Reply::Error("Invalid Page".to_string())).await;
                            continue;
                        }
                        if let Some(query) = path.strip_suffix("?") {
                            let query_path = page_dir.join(query);
                            if query_path.exists() && query_path.is_dir() {
                                let _ = req
                                    .reply(match list_dir(&query_path).await {
                                        Ok(entries) => Text(to_string(&entries).unwrap_or_default()),
                                        Err(e) => Reply::Error(e.to_string()),
                                    })
                                .await;
                            } else {
                                let _ = req.reply(Text(query_path.exists().to_string())).await;
                            }
                            continue;
                            // TODO:: glob
                        }
                        let asset_path = page_dir.join(path);
                        if !asset_path.exists() {
                            let _ = req.reply(Reply::Error("Invalid Path".to_string())).await;
                            continue;
                        }
                        let Ok(mut asset_file) = File::open(&asset_path).await else {
                            let _ = req
                                .reply(Reply::Error(format!("Cannot Open {asset_path}")))
                                .await;
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
                                let _ = req.reply(Text(format!("!{e}"))).await;
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
