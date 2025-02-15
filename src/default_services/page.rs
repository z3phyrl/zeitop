use super::DefaultService;
use crate::config::Config;
use crate::service::{Reply, Request, RequestService};
use anyhow::{Error, Result};
use futures::{future, pin_mut, SinkExt, StreamExt};
use sass_rs::{compile_string, Options};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::fs;

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
                    if req.request == "default" {
                        let default_page = to_string(&Page::default()).unwrap_or_default();
                        let _ = req.reply(Reply::Text(default_page.to_string())).await;
                    } else {
                        unimplemented!()
                    }
                }
            }
        });
        Ok(())
    }
}

impl Default for Page {
    fn default() -> Self {
        let default = Config::default_dir().unwrap().join("pages/default/");
        if default.exists() {
            let name = "default";
            let content = fs::read_to_string(default.join("page.html")).unwrap_or_default();
            let scss = fs::read_to_string(default.join("style.scss")).unwrap_or_default();
            let style = compile_string(&scss, Options::default()).unwrap_or_default();
            println!("{style:?}");
            let script = fs::read_to_string(default.join("script.js")).unwrap_or_default();
            return Self {
                name: String::from(name),
                content,
                style,
                script,
            };
        }
        panic!();
    }
}
