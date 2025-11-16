use super::DefaultService;
use anyhow::Result;
use mpd_client::{
    Client,
    client::ConnectionEvent,
    commands::{CurrentSong, Next, Play, Previous, SetPause, Status},
    tag::Tag,
};
use serde::Serialize;
use serde_json::to_string;
use std::collections::HashMap;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::task::spawn_blocking;

use crate::service::{BroadcastMessage, BroadcastService, Reply, RequestService};

pub struct MpdService {}

#[derive(Serialize)]
struct SongInfo {
    title: Option<String>,
    artists: Vec<String>,
    album: Option<String>,
    album_artists: Vec<String>,
}
#[derive(Serialize)]
enum PlayState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Serialize)]
struct StatusSer {
    volume: u8,
    state: PlayState,
    repeat: bool,
    random: bool,
    consume: bool,
    elapsed: Option<Duration>,
    duration: Option<Duration>,
}

impl DefaultService for MpdService {
    async fn run() -> Result<()> {
        let mut mpdctl = RequestService::new("mpd").await?;
        let mpdevents = BroadcastService::new("mpd-events").await?;
        let stream = TcpStream::connect("127.0.0.1:6600").await.unwrap();
        let (mpd, mut event) = Client::connect(stream).await.unwrap();
        tokio::spawn(async move {
            loop {
                if let Some(req) = mpdctl.next().await {
                    match req.request.as_str() {
                        "play" => {
                            let _ = req
                                .reply(Reply::Text(
                                    mpd.command(SetPause(false)).await.is_ok().to_string(),
                                ))
                                .await;
                        }
                        "pause" => {
                            let _ = req
                                .reply(Reply::Text(
                                    mpd.command(SetPause(true)).await.is_ok().to_string(),
                                ))
                                .await;
                        }
                        "next" => {
                            let _ = req
                                .reply(Reply::Text(mpd.command(Next).await.is_ok().to_string()))
                                .await;
                        }
                        "prev" => {
                            let _ = req
                                .reply(Reply::Text(mpd.command(Previous).await.is_ok().to_string()))
                                .await;
                        }
                        "currentsong" => {
                            let Ok(Some(currentsong)) = mpd.command(CurrentSong).await else {
                                let _ = req
                                    .reply(Reply::Error("Error Current Song Unavailable"))
                                    .await;
                                continue;
                            };
                            let songinfo = SongInfo {
                                title: currentsong.song.title().map(|t| t.to_owned()),
                                artists: currentsong.song.artists().to_vec(),
                                album: currentsong.song.album().map(|a| a.to_owned()),
                                album_artists: currentsong.song.album_artists().to_vec(),
                            };
                            let _ = req
                                .reply(Reply::Text(to_string(&songinfo).unwrap_or_default()))
                                .await;
                        }
                        "status" => {
                            let Ok(status) = mpd.command(Status).await else {
                                let _ = req.reply(Reply::Error("Error Status Unavailable")).await;
                                continue;
                            };
                            let statusser = StatusSer {
                                volume: status.volume,
                                state: match status.state {
                                    mpd_client::responses::PlayState::Stopped => PlayState::Stopped,
                                    mpd_client::responses::PlayState::Playing => PlayState::Playing,
                                    mpd_client::responses::PlayState::Paused => PlayState::Paused,
                                },
                                repeat: status.repeat,
                                random: status.random,
                                consume: status.consume,
                                elapsed: status.elapsed,
                                duration: status.duration,
                            };
                            let _ = req
                                .reply(Reply::Text(to_string(&statusser).unwrap_or_default()))
                                .await;
                        }
                        request => {
                            let _ = req.reply(Reply::Error("Invalid Request")).await;
                            println!("Requested => {request} :: Unavailable");
                        }
                    }
                }
            }
        });
        tokio::spawn(async move {
            loop {
                match event.next().await {
                    Some(ConnectionEvent::SubsystemChange(subsystem)) => {
                        println!("{subsystem:?}");
                        let _ = mpdevents
                            .broadcast(BroadcastMessage::Text(format!("{subsystem:?}")))
                            .await;
                    }
                    Some(ConnectionEvent::ConnectionClosed(e)) => {
                        eprintln!("{e}");
                    }
                    None => {}
                }
            }
        });
        Ok(())
    }
}
