use super::DefaultService;
use crate::service::{BroadcastMessage, BroadcastService, Reply, RequestService};
use anyhow::Result;
use serde_json::to_string;
use std::collections::HashMap;
use std::process;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use sysinfo::{Components, Disks, Networks, Pid, System, Users};
use tokio::task::spawn_blocking;

pub struct SysInfoService {}

impl DefaultService for SysInfoService {
    async fn run() -> Result<()> {
        let mut request = RequestService::new("sysinfo").await?;
        tokio::spawn(async move {
            let mut sys = System::new_all();
            let users = Users::new_with_refreshed_list();
            loop {
                if let Some(req) = request.next().await {
                    sys.refresh_cpu_all();
                    sys.refresh_memory();
                    match req.request.as_str() {
                        "user" => {
                            let this = sys.process(Pid::from_u32(process::id())).unwrap();
                            let user = users
                                .get_user_by_id(this.user_id().unwrap())
                                .unwrap()
                                .name()
                                .to_string();
                            let _ = req.reply(Reply::Text(user.clone())).await;
                        }
                        "host" => {
                            let _ = req
                                .reply(Reply::Text(System::host_name().unwrap_or_default()))
                                .await;
                        }
                        "cpu" => {
                            let cpus: HashMap<String, f32> = sys
                                .cpus()
                                .iter()
                                .map(|c| (String::from(c.name()), c.cpu_usage()))
                                .collect();
                            let _ = req.reply(Reply::Text(to_string(&cpus).unwrap())).await;
                        }
                        "total_mem" => {
                            let _ = req.reply(Reply::Text(sys.total_memory().to_string())).await;
                        }
                        "used_mem" => {
                            let _ = req.reply(Reply::Text(sys.used_memory().to_string())).await;
                        }
                        "uptime" => {
                            let _ = req.reply(Reply::Text(System::uptime().to_string())).await;
                        }
                        request => {
                            let _ = req.reply(Reply::Error("Invalid Request".to_string())).await;
                            println!("Requested => {request} :: Unavailable");
                        }
                    }
                }
            }
        });
        Ok(())
    }
}
