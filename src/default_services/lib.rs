use super::DefaultService;
use crate::config::Config;
use crate::service::{Reply, RequestService};
use anyhow::{Error, Result};
use include_dir::{Dir, include_dir};
use tokio::{fs::read_to_string, spawn};

static DEFAULT_SERVICES_LIBS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/default_services/libs/");

pub struct LibService;

impl DefaultService for LibService {
    async fn run() -> Result<()> {
        let mut lib = RequestService::new("lib").await?;
        let lib_path = Config::dir().join("libs/");
        spawn(async move {
            loop {
                if let Some(req) = lib.next().await {
                    let lib_path = lib_path.join(format!("{}.js", &req.request));
                    let lib = if lib_path.exists() {
                        read_to_string(lib_path.join(format!("{}.js", &req.request)))
                            .await
                            .unwrap_or_default()
                    } else if let Some(lib) =
                        DEFAULT_SERVICES_LIBS.get_file(format!("{}.js", &req.request))
                    {
                        lib.contents_utf8().unwrap_or_default().to_string()
                    } else {
                        let _ = req.reply(Reply::Error("Invalid Path")).await;
                        continue;
                    };
                    let _ = req.reply(Reply::Text(lib)).await;
                }
            }
        });
        Ok(())
    }
}
