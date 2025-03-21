use crate::device::Serial;
use crate::server::{Connection, ConnectionIO, ConnectionMap};
use crate::service::{BroadcastHandler, ServiceMapExt, ServiceType};
use anyhow::{Error, Result};
use futures::{
    prelude::stream::{SplitSink, SplitStream},
    Sink, SinkExt, Stream, StreamExt,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::spawn_blocking;
use tokio::time::interval;
use tokio_tungstenite::WebSocketStream;
use tungstenite::Message;

pub type ClientMap = Arc<RwLock<HashMap<Serial, BTreeMap<u32, Client>>>>;

pub trait ClientMapExt {
    async fn insert(&self, serial: Serial, client: Client) -> Result<u32>;
    async fn get(&self, serial: Serial, id: u32) -> Option<Client>;
    async fn remove(&self, serial: Serial, id: u32) -> Result<()>;
}

impl ClientMapExt for ClientMap {
    async fn insert(&self, serial: Serial, client: Client) -> Result<u32> {
        let this = self.clone();
        spawn_blocking(move || {
            let mut this = this.write().unwrap();
            if let Some(map) = this.get_mut(&serial) {
                let last_id: u32 = *map.keys().last().unwrap_or(&0);
                map.insert(last_id + 1, client);
                Ok(last_id + 1)
            } else {
                let mut map = BTreeMap::new();
                let last_id: u32 = *map.keys().last().unwrap_or(&0);
                map.insert(last_id + 1, client);
                this.insert(serial, map);
                Ok(last_id + 1)
            }
        })
        .await?
    }
    async fn get(&self, serial: Serial, id: u32) -> Option<Client> {
        let this = self.clone();
        if let Ok(client) = spawn_blocking(move || {
            if let Some(clients) = this.read().unwrap().get(&serial) {
                clients.get(&id).cloned()
            } else {
                None
            }
        })
        .await
        {
            client
        } else {
            None
        }
    }
    async fn remove(&self, serial: Serial, id: u32) -> Result<()> {
        let this = self.clone();
        spawn_blocking(move || {
            let mut that = this.write().unwrap();
            if let Some(clients) = that.get_mut(&serial) {
                clients.remove(&id);
                if clients.len() < 1 {
                    that.remove(&serial);
                }
                Ok(())
            } else {
                Err(Error::msg("Invalid Client Serial"))
            }
        })
        .await?
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    serial: Serial,
    connection: Connection,
}

impl ConnectionIO for Client {
    async fn read(&mut self) -> Result<Message> {
        self.connection.read().await
    }
    fn send(&self, msg: Message) -> Result<()> {
        self.connection.send(msg)
    }
}

pub struct ClientHandler {
    client: Client,
    id: u32,
    connection_map: ConnectionMap,
}

impl ClientHandler {
    pub async fn new(client: Client, connection_map: ConnectionMap) -> Result<Self> {
        let id = connection_map
            .client_map
            .insert(client.serial.clone(), client.clone())
            .await?;
        Ok(Self {
            client,
            id,
            connection_map,
        })
    }
    pub async fn from_req(
        req: &str,
        connection: &Connection,
        connection_map: &ConnectionMap,
    ) -> Result<Self> {
        let client = Client {
            serial: String::from(req),
            connection: connection.clone(),
        };
        let _ = client.send(Message::text("@Ok"));
        Self::new(client, connection_map.clone()).await
    }
    pub async fn handle(&mut self) -> Result<()> {
        match self.read().await {
            Ok(Message::Close(_f)) => {
                if self
                    .connection_map
                    .client_map
                    .remove(self.client.serial.clone(), self.id)
                    .await
                    .is_ok()
                {
                    Err(Error::msg("Connection closed"))
                } else {
                    Err(Error::msg("Connection should close but not"))
                }
            }
            Ok(Message::Text(req)) => {
                println!("{req}");
                if let Some(service_req) = req.strip_prefix("&") {
                    let mut req = service_req.splitn(2, "::");
                    let Some((service_name, tag)) = req.next().map(|r| {
                        r.split_once("#")
                            .map(|(s, t)| (s, format!("#{t}")))
                            .unwrap_or((r, String::new()))
                    }) else {
                        let _ = self.send("!Service Name Unspecified".into());
                        return Ok(());
                    };
                    let Ok(service) = self.connection_map.service_map.get(service_name).await
                    else {
                        let _ = self.send("!Invalid Service".into());
                        return Ok(());
                    };
                    match service.service_type {
                        ServiceType::Request => {
                            if let Some(req) = req.next() {
                                if service
                                    .send(
                                        format!(
                                            "{}@{}{}::{}",
                                            self.client.serial, self.id, tag, req
                                        )
                                        .into(),
                                    )
                                    .is_err()
                                {
                                    return Err(Error::msg("Can not Send"));
                                }
                            }
                        }
                        ServiceType::Broadcast => {
                            let Ok(mut handler) = BroadcastHandler::new(
                                service,
                                self.client.clone(),
                                self.connection_map.clone(),
                            ) else {
                                let _ = self.send("!Not a Broadcast Service".into());
                                return Ok(());
                            };
                            tokio::spawn(async move {
                                loop {
                                    if let Err(e) = handler.handle().await {
                                        eprintln!("{e}");
                                        break;
                                    }
                                }
                            });
                        }
                    }
                } else {
                    let _ = self.send("!Invalid Request".into());
                }
                Ok(())
            }
            Ok(Message::Binary(bytes)) => {
                unimplemented!()
            }
            Ok(msg) => {
                println!("{msg}");
                Ok(())
            }
            Err(e) => {
                eprintln!("{e}");
                Err(e)
            }
        }
    }
}

impl ConnectionIO for ClientHandler {
    async fn read(&mut self) -> Result<Message> {
        self.client.read().await
    }
    fn send(&self, msg: Message) -> Result<()> {
        self.client.send(msg)
    }
}
