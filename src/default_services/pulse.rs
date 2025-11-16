use super::DefaultService;
use crate::service::{Reply, RequestService};
use anyhow::Result;
use tokio::process::Command;

pub struct PulseAudioService;

impl DefaultService for PulseAudioService {
    async fn run() -> Result<()> {
        let mut request = RequestService::new("pulse").await?;

        tokio::spawn(async move {
            loop {
                if let Some(req) = request.next().await {
                    let parts: Vec<&str> = req.request.split_whitespace().collect();
                    let cmd = parts.get(0).copied().unwrap_or("");

                    let reply = match cmd {
                        // ----------------------
                        // SINK VOLUME CONTROL
                        // ----------------------
                        "vol_get_sink" => match get_default_sink_volume().await {
                            Ok(v) => Reply::Text(v.to_string()),
                            Err(e) => Reply::Error(e.to_string()),
                        },

                        "vol_set_sink" => {
                            if let Some(val) = parts.get(1) {
                                Reply::Text(set_default_sink_volume(val).await.to_string())
                            } else {
                                Reply::Error("usage: vol_set_sink <0-150>".into())
                            }
                        }

                        "vol_inc_sink" => {
                            if let Some(val) = parts.get(1) {
                                Reply::Text(inc_default_sink_volume(val).await.to_string())
                            } else {
                                Reply::Error("usage: vol_inc_sink <percent>".into())
                            }
                        }

                        "vol_dec_sink" => {
                            if let Some(val) = parts.get(1) {
                                Reply::Text(dec_default_sink_volume(val).await.to_string())
                            } else {
                                Reply::Error("usage: vol_dec_sink <percent>".into())
                            }
                        }

                        "vol_mute_sink" => Reply::Text(mute_default_sink(true).await.to_string()),

                        "vol_unmute_sink" => {
                            Reply::Text(mute_default_sink(false).await.to_string())
                        }

                        // ----------------------
                        // SOURCE (MIC) CONTROL
                        // ----------------------
                        "vol_mute_mic" => Reply::Text(mute_default_source(true).await.to_string()),

                        "vol_unmute_mic" => {
                            Reply::Text(mute_default_source(false).await.to_string())
                        }

                        other => Reply::Error(format!("Unknown pulseaudio cmd: {other}")),
                    };

                    let _ = req.reply(reply).await;
                }
            }
        });

        Ok(())
    }
}

async fn pactl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("pactl")
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

async fn default_sink() -> Result<String, String> {
    Ok(pactl(&["get-default-sink"]).await?.trim().to_string())
}

async fn default_source() -> Result<String, String> {
    Ok(pactl(&["get-default-source"]).await?.trim().to_string())
}

async fn get_default_sink_volume() -> Result<u32, String> {
    let sink = default_sink().await?;
    let out = pactl(&["get-sink-volume", &sink]).await?;
    // parse "Volume: front-left: 32768 /  50% ..."
    let percent = out
        .split('/')
        .nth(1)
        .ok_or("parse error")?
        .trim()
        .trim_end_matches('%')
        .parse::<u32>()
        .map_err(|_| "parse int".to_string())?;
    Ok(percent)
}

async fn set_default_sink_volume(percent: &str) -> bool {
    if let Ok(sink) = default_sink().await {
        pactl(&["set-sink-volume", &sink, &format!("{percent}%")])
            .await
            .is_ok()
    } else {
        false
    }
}

async fn inc_default_sink_volume(amount: &str) -> bool {
    if let Ok(sink) = default_sink().await {
        pactl(&["set-sink-volume", &sink, &format!("+{amount}%")])
            .await
            .is_ok()
    } else {
        false
    }
}

async fn dec_default_sink_volume(amount: &str) -> bool {
    if let Ok(sink) = default_sink().await {
        pactl(&["set-sink-volume", &sink, &format!("-{amount}%")])
            .await
            .is_ok()
    } else {
        false
    }
}

async fn mute_default_sink(mute: bool) -> bool {
    if let Ok(sink) = default_sink().await {
        pactl(&["set-sink-mute", &sink, if mute { "1" } else { "0" }])
            .await
            .is_ok()
    } else {
        false
    }
}

async fn mute_default_source(mute: bool) -> bool {
    if let Ok(src) = default_source().await {
        pactl(&["set-source-mute", &src, if mute { "1" } else { "0" }])
            .await
            .is_ok()
    } else {
        false
    }
}
