use super::DefaultService;
use crate::service::{Reply, RequestService};
use anyhow::Result;
use obws::client::Client;
use obws::requests::{inputs::InputId, scenes::SceneId};

pub struct ObsService;

impl DefaultService for ObsService {
    async fn run() -> Result<()> {
        // This connects once when the service starts.
        // Adjust host/port/password to your OBS websocket config.
        let mut request = RequestService::new("obs").await?;

        tokio::spawn(async move {
            let client = match Client::connect("127.0.0.1", 4455, Some("1234567809")).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("OBS connect error: {e:?}");
                    return;
                }
            };

            loop {
                if let Some(req) = request.next().await {
                    let mut parts = req.request.split_whitespace();
                    let cmd = parts.next().unwrap_or("");

                    match cmd {
                        // --- Recording state / control ---

                        // "record_state" -> "recording" | "paused" | "stopped"
                        "record_state" => match client.recording().status().await {
                            Ok(status) => {
                                let state = if status.active {
                                    if status.paused { "paused" } else { "recording" }
                                } else {
                                    "stopped"
                                };
                                let _ = req.reply(Reply::Text(state.to_string())).await;
                            }
                            Err(e) => {
                                let _ = req
                                    .reply(Reply::Error(format!("record_state failed: {e}")))
                                    .await;
                            }
                        },

                        // "record_start"
                        "record_start" => {
                            let ok = client.recording().start().await.is_ok();
                            let _ = req.reply(Reply::Text(ok.to_string())).await;
                        }

                        // "record_stop"
                        "record_stop" => {
                            // OBS API returns the file name, but we just map to bool.
                            let ok = client.recording().stop().await.is_ok();
                            let _ = req.reply(Reply::Text(ok.to_string())).await;
                        }

                        // --- Scene switching ---

                        // "scene Some Scene Name"
                        "scene" => {
                            let scene_name = parts.collect::<Vec<_>>().join(" ");
                            if scene_name.is_empty() {
                                let _ = req
                                    .reply(Reply::Error("missing scene name".to_string()))
                                    .await;
                                continue;
                            }

                            // SceneId implements From<&str>, and set_current_program_scene
                            // takes impl Into<SceneId<'_>>.
                            let scene_id: SceneId<'_> = SceneId::from(scene_name.as_str());
                            let ok = client
                                .scenes()
                                .set_current_program_scene(scene_id)
                                .await
                                .is_ok();
                            let _ = req.reply(Reply::Text(ok.to_string())).await;
                        }

                        // --- Mic / Desktop mute control using special inputs ---

                        // "mic_mute" / "mic_unmute" -> mic1 from Inputs::specials()
                        "mic_mute" | "mic_unmute" => {
                            let want_mute = cmd == "mic_mute";
                            match client.inputs().specials().await {
                                Ok(specials) => {
                                    if let Some(name) = specials.mic1 {
                                        let id: InputId<'_> = InputId::from(name.as_str());
                                        let ok =
                                            client.inputs().set_muted(id, want_mute).await.is_ok();
                                        let _ = req.reply(Reply::Text(ok.to_string())).await;
                                    } else {
                                        let _ = req
                                            .reply(Reply::Error(
                                                "no mic1 special input".to_string(),
                                            ))
                                            .await;
                                    }
                                }
                                Err(e) => {
                                    let _ = req
                                        .reply(Reply::Error(format!("specials() failed: {e}")))
                                        .await;
                                }
                            }
                        }

                        // "desktop_mute" / "desktop_unmute" -> desktop1 from Inputs::specials()
                        "desktop_mute" | "desktop_unmute" => {
                            let want_mute = cmd == "desktop_mute";
                            match client.inputs().specials().await {
                                Ok(specials) => {
                                    if let Some(name) = specials.desktop1 {
                                        let id: InputId<'_> = InputId::from(name.as_str());
                                        let ok =
                                            client.inputs().set_muted(id, want_mute).await.is_ok();
                                        let _ = req.reply(Reply::Text(ok.to_string())).await;
                                    } else {
                                        let _ = req
                                            .reply(Reply::Error(
                                                "no desktop1 special input".to_string(),
                                            ))
                                            .await;
                                    }
                                }
                                Err(e) => {
                                    let _ = req
                                        .reply(Reply::Error(format!("specials() failed: {e}")))
                                        .await;
                                }
                            }
                        }

                        // --- Fallback ---
                        other => {
                            let _ = req
                                .reply(Reply::Error(format!("Invalid OBS request: {other}")))
                                .await;
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
