use crate::client::{Client, ClientMapExt};
use crate::config::Config;
use crate::device::Serial;
use crate::server::{Connection, ConnectionIO, ConnectionMap};
use anyhow::{Error, Result};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::spawn_blocking;
use tokio_tungstenite::connect_async;
use tungstenite::Message;

pub type ServiceMap = Arc<RwLock<HashMap<String, Service>>>;

pub trait ServiceMapExt {
    async fn insert(&self, name: impl Into<String>, service: Service) -> Result<()>;
    async fn get(&self, name: impl Into<String>) -> Result<Service>;
    async fn remove(&self, name: impl Into<String>) -> Result<()>;
}

impl ServiceMapExt for ServiceMap {
    async fn insert(&self, name: impl Into<String>, service: Service) -> Result<()> {
        let name = name.into();
        let this = self.clone();
        spawn_blocking(move || {
            let mut this = this.write().unwrap();
            if this.get(&name).is_none() {
                this.insert(name, service);
                Ok(())
            } else {
                Err(Error::msg("Service already registered"))
            }
        })
        .await?
    }
    async fn get(&self, name: impl Into<String>) -> Result<Service> {
        let name = name.into();
        let this = self.clone();
        spawn_blocking(move || {
            let this = this.read().unwrap();
            let Some(service) = this.get(&name) else {
                return Err(Error::msg("Invalid Service"));
            };
            Ok(service.clone())
        })
        .await?
    }
    async fn remove(&self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        let this = self.clone();
        Ok(spawn_blocking(move || {
            this.write().unwrap().remove(&name);
        })
        .await?)
    }
}

#[derive(Debug, Clone)]
pub enum ServiceType {
    Request,
    Broadcast,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub service_type: ServiceType,
    connection: Connection,
}

impl ConnectionIO for Service {
    async fn read(&mut self) -> Result<Message> {
        self.connection.read().await
    }
    fn send(&self, msg: Message) -> Result<()> {
        self.connection.send(msg)
    }
}

impl Service {
    pub fn from_req(req: &str, connection: &Connection) -> Result<Option<Self>> {
        let Some(service_add) = req.strip_prefix("+") else {
            return Ok(None);
        };
        let mut req = service_add.split("::");
        let Some(name) = req.next() else {
            connection.send(Message::text("Service Name Unspecified"))?;
            return Ok(None);
        };
        match req.next() {
            Some("request") => Ok(Some(Self {
                name: String::from(name),
                service_type: ServiceType::Request,
                connection: connection.clone(),
            })),
            Some("broadcast") => Ok(Some(Self {
                name: String::from(name),
                service_type: ServiceType::Broadcast,
                connection: connection.clone(),
            })),
            Some(_) => {
                connection.send(Message::text("Invalid ServiceType"))?;
                Err(Error::msg("Invalid ServiceType"))
            }
            None => {
                connection.send(Message::text("ServiceType Unspecified"))?;
                Err(Error::msg("ServiceType Unspecified"))
            }
        }
    }
}

pub struct RequestHandler {
    pub service: Service,
    connection_map: ConnectionMap,
}

impl RequestHandler {
    pub fn new(service: Service, connection_map: ConnectionMap) -> Result<Self> {
        if let ServiceType::Request = service.service_type {
            Ok(Self {
                service,
                connection_map,
            })
        } else {
            Err(Error::msg("Not a Request Service"))
        }
    }
    pub async fn handle(&mut self) -> Result<()> {
        match self.read().await {
            Ok(Message::Close(_)) => {
                if self
                    .connection_map
                    .service_map
                    .remove(&self.service.name)
                    .await
                    .is_ok()
                {
                    Err(Error::msg("Connection closed"))
                } else {
                    Err(Error::msg("Connection should but not"))
                }
            }
            Ok(Message::Text(req)) => {
                let mut req = req.splitn(2, "::");
                let Some(Some((serial, Some((Ok(id), (request, tag)))))) = req.next().map(|r| {
                    r.split_once("@").map(|(s, d)| {
                        (
                            s,
                            d.split_once("&").map(|(i, r)| {
                                (
                                    i.parse::<u32>(),
                                    r.split_once("#")
                                        .map(|(r, t)| (r, format!("#{t}")))
                                        .unwrap_or((r, String::new())),
                                )
                            }),
                        )
                    })
                    // unknown@1&req#tag::data
                }) else {
                    let _ = self.send("!Invalid Destination".into());
                    return Ok(());
                };
                if let Some(data) = req.next() {
                    let Some(client) = self
                        .connection_map
                        .client_map
                        .get(String::from(serial), id)
                        .await
                    else {
                        let _ = self.send("!Invalid Destination".into());
                        return Ok(());
                    };
                    client
                        .send(format!("{request}{tag}@{}::{data}", self.service.name).into())
                        .unwrap();
                }
                Ok(())
            }
            Ok(Message::Binary(_bytes)) => {
                unimplemented!();
            }
            Ok(msg) => {
                eprintln!("{msg}");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl ConnectionIO for RequestHandler {
    async fn read(&mut self) -> Result<Message> {
        self.service.read().await
    }
    fn send(&self, msg: Message) -> Result<()> {
        self.service.send(msg)
    }
}

pub struct BroadcastHandler {
    service: Service,
    client: Client,
    connection_map: ConnectionMap,
}

impl BroadcastHandler {
    pub fn new(service: Service, client: Client, connection_map: ConnectionMap) -> Result<Self> {
        if let ServiceType::Broadcast = service.service_type {
            Ok(Self {
                service,
                client,
                connection_map,
            })
        } else {
            Err(Error::msg("Not a Broadcast Service"))
        }
    }
    pub async fn handle(&mut self) -> Result<()> {
        match self.read().await {
            Ok(Message::Close(_f)) => {
                if self
                    .connection_map
                    .service_map
                    .remove(&self.service.name)
                    .await
                    .is_ok()
                {
                    Err(Error::msg("Connection closed"))
                } else {
                    Err(Error::msg("Connection should but not"))
                }
            }
            Ok(Message::Text(req)) => {
                if self
                    .client
                    .send(format!("{}::{}", self.service.name, req.as_str()).into())
                    .is_err()
                {
                    return Err(Error::msg("Can not send"));
                };
                Ok(())
            }
            Ok(Message::Binary(bytes)) => {
                unimplemented!()
            }
            Ok(msg) => {
                // println!("{msg:?}");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl ConnectionIO for BroadcastHandler {
    async fn read(&mut self) -> Result<Message> {
        self.service.read().await
    }
    fn send(&self, msg: Message) -> Result<()> {
        self.service.send(msg)
    }
}

pub struct RequestService {
    connection: Connection,
}

pub struct BroadcastService {
    connection: Connection,
}

pub struct ClientInfo {
    serial: Serial,
    id: u32,
}

pub struct Request {
    reply_channel: UnboundedSender<Message>,
    info: ClientInfo,
    tag: String,
    pub request: String,
}

pub enum Reply<T>
where
    T: Into<String>,
{
    Text(T),
    Binary(Bytes),
    Error(T),
}

pub enum BroadcastMessage {
    Text(String),
    Binary(Bytes),
}

impl RequestService {
    pub async fn new(name: &str) -> Result<Self> {
        let port = 6969; // TODO :: config thingy i'm too lazy for all of that
        let (ws, _) = connect_async(format!("ws://localhost:{}", port)).await?;
        let (mut sink, stream) = ws.split();
        sink.send(Message::text(format!("+{name}::request")))
            .await?;
        Ok(Self {
            connection: Connection::new(stream, sink).await,
        })
    }
    pub async fn next(&mut self) -> Option<Request> {
        match self.connection.read().await {
            Ok(Message::Text(req)) => {
                if req == "?" {
                    let _ = self.connection.send(Message::text("?"));
                    return None;
                }
                let mut req = req.splitn(2, "::");
                let Some(Some((serial, (Ok(id), tag)))) = req.next().map(|r| {
                    r.split_once("@").map(|(s, i)| {
                        (
                            s,
                            i.split_once("#")
                                .map(|(i, t)| (i.parse::<u32>(), format!("#{t}")))
                                .unwrap_or((i.parse::<u32>(), String::new())),
                        )
                    })
                }) else {
                    return None;
                };
                if let Some(req) = req.next() {
                    return Some(Request {
                        reply_channel: self.connection.sender.clone(),
                        info: ClientInfo {
                            serial: String::from(serial),
                            id,
                        },
                        tag,
                        request: String::from(req),
                    });
                }
                None
            }
            Ok(Message::Binary(bytes)) => {
                unimplemented!()
            }
            Ok(msg) => None,
            Err(e) => None,
        }
    }
}

// TODO::impl Stream and StreamExt instead

impl Request {
    pub async fn reply<T>(&self, reply: Reply<T>) -> Result<()>
    where
        T: Into<String>,
    {
        match reply {
            Reply::Text(reply) => {
                let reply = reply.into();
                self.reply_channel.send(Message::text(format!(
                    "{}@{}&{}{}::{}",
                    self.info.serial, self.info.id, self.request, self.tag, reply
                )))?;
            }
            Reply::Binary(bytes) => {
                unimplemented!()
            }
            Reply::Error(e) => {
                let e = e.into();
                println!("heh => {e}");
                self.reply_channel.send(Message::text(format!(
                    "{}@{}&{}{}::!{}",
                    self.info.serial, self.info.id, self.request, self.tag, e
                )))?;
            }
        }
        Ok(())
    }
}

impl BroadcastService {
    pub async fn new(name: &str) -> Result<Self> {
        let port = 6969; // TODO :: config thingy i'm too lazy for all of that
        let (ws, _) = connect_async(format!("ws://localhost:{}", port)).await?;
        let (mut sink, stream) = ws.split();
        sink.send(Message::text(format!("+{name}::broadcast")))
            .await?;
        Ok(Self {
            connection: Connection::new(stream, sink).await,
        })
    }
    pub async fn broadcast(&self, message: BroadcastMessage) -> Result<()> {
        match message {
            BroadcastMessage::Text(msg) => self.connection.send(Message::text(msg))?,
            BroadcastMessage::Binary(bytes) => {
                unimplemented!()
            }
        }
        Ok(())
    }
}

// TODO :: send request@service::data or somthing of that sort
//      :: point is to include more infomations for client to know
//      :: and maybe let client tag their own info for idntification
