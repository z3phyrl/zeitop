use super::DefaultService;
use crate::config::Config;
use crate::service::{Reply, Request, RequestService};
use anyhow::{Error, Result};
use sass_rs::{compile_string, Options};
use serde::Serialize;
use serde_json::to_string;
use tokio::{fs::read_to_string, task::spawn_blocking};

#[derive(Serialize, Clone)]
struct Page {
    name: String,
    content: String,
    style: String,
    script: String,
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
                    println!("{}", req.request);
                    match Page::load(&req.request).await {
                        Ok(page) => {
                            let _ = req
                                .reply(Reply::Text(to_string(&page).unwrap_or_default()))
                                .await;
                        }
                        Err(e) => {
                            let _ = req.reply(Reply::Text(format!("!{e}")));
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
        let content = read_to_string(page_dir.join("page.html"))
            .await
            .unwrap_or_default();
        let scss = read_to_string(page_dir.join("style.scss"))
            .await
            .unwrap_or_default();
        let compile = spawn_blocking(move || compile_string(&scss, Options::default()));
        let script = read_to_string(page_dir.join("script.js"))
            .await
            .unwrap_or_default();
        let style = compile.await?.unwrap_or_default();
        Ok(Self {
            name: String::from(name),
            content,
            style,
            script,
        })
    }
}
