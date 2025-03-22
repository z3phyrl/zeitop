// use crate::device::{Device, DeviceMap, Serial};
use anyhow::{Error, Result};
use bytes::Bytes;
use futures::{
    prelude::stream::{SplitSink, SplitStream},
    Sink, SinkExt, Stream, StreamExt,
};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufWriter, Read, Write};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::spawn_blocking;
use tokio::time::interval;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::WebSocketStream;
use tungstenite::protocol::{frame::Frame, frame::Utf8Bytes, Message};

use crate::client::{Client, ClientHandler, ClientMap};
use crate::service::{RequestHandler, Service, ServiceMap, ServiceMapExt, ServiceType};

pub struct Server {
    listener: TcpListener,
    connection_map: ConnectionMap,
}

#[derive(Clone, Debug)]
pub struct ConnectionMap {
    pub client_map: ClientMap,
    pub service_map: ServiceMap,
}

impl Server {
    pub async fn new(port: u16) -> Result<Self> {
        let listener = TcpListener::bind(format!("localhost:{port}")).await?;
        let client_map = Arc::new(RwLock::new(HashMap::new()));
        let service_map = Arc::new(RwLock::new(HashMap::new()));
        let connection_map = ConnectionMap {
            client_map,
            service_map,
        };
        println!("Server => Bind :: localhost:{port}");
        Ok(Self {
            listener,
            connection_map,
        })
    }
    pub async fn handle(&self) {
        if let Ok((raw_stream, addr)) = self.listener.accept().await {
            println!("Server => Connect :: {addr}");
            let connection_map = self.connection_map.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::accept_ws(raw_stream, connection_map).await {
                    eprintln!("{e}");
                }
            });
        }
        {
            println!(
                "ClientMap :: {:#?}",
                self.connection_map.client_map.read().unwrap()
            );
            println!(
                "ServiceMap :: {:#?}",
                self.connection_map.service_map.read().unwrap()
            );
        }
    }
    async fn accept_ws(raw_stream: TcpStream, connection_map: ConnectionMap) -> Result<()> {
        match accept_async(raw_stream).await {
            Ok(ws) => {
                let (sink, stream) = ws.split();
                let mut connection = Connection::new(stream, sink).await;
                loop {
                    if let Ok(Message::Text(req)) = connection.read().await {
                        if let Some(service) = Service::from_req(req.as_str(), &connection)? {
                            if let Err(e) = connection_map
                                .service_map
                                    .insert(&service.name, service.clone())
                                    .await
                            {
                                service.send(format!("!{e}").into())?;
                                return Ok(());
                            };
                            println!("Service => {} :: {:?}", service.name, service.service_type);
                            if let ServiceType::Request = service.service_type {
                                let mut handler = RequestHandler::new(service, connection_map)?;
                                tokio::spawn(async move {
                                    loop {
                                        if let Err(e) = handler.handle().await {
                                            eprintln!("{e}");
                                            break;
                                        }
                                    }
                                });
                            }
                            break;
                        } else if let Ok(mut client_handler) =
                            ClientHandler::from_req(req.as_str(), &connection, &connection_map).await
                        {
                            tokio::spawn(async move {
                                loop {
                                    if let Err(e) = client_handler.handle().await {
                                        eprintln!("{e}");
                                        break;
                                    }
                                }
                            });
                            break;
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug)]
pub struct Connection {
    pub sender: UnboundedSender<Message>,
    broadcast: broadcast::Sender<Message>,
    receiver: broadcast::Receiver<Message>,
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            broadcast: self.broadcast.clone(),
            receiver: self.broadcast.subscribe(),
        }
    }
}

impl Connection {
    pub async fn new<S>(
        stream: SplitStream<WebSocketStream<S>>,
        mut sink: SplitSink<WebSocketStream<S>, Message>,
    ) -> Self where S: AsyncRead + AsyncWrite + Unpin + Send + 'static {
        let (tx, mut rx) = unbounded_channel::<Message>();
        let (b, r) = broadcast::channel(64);
        tokio::spawn(async move {
            loop {
                if let Some(msg) = rx.recv().await {
                    let _ = sink.send(msg).await;
                }
            }
        });
        let sndr = b.clone();
        tokio::spawn(async move {
            stream
                .for_each(|msg| async {
                    if let Ok(msg) = msg {
                        if let Message::Text(ref text) = msg {
                            if text.as_str() == "?" {
                                // println!("ping");
                                return;
                            }
                        }
                        let _ = sndr.send(msg);
                    }
                })
                .await;
        });
        let ping_tx = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let mut ping = interval(Duration::from_secs(30));
            loop {
                ping.tick().await;
                let _ = ping_tx.send("?".into());
            }
        });
        Self {
            sender: tx,
            broadcast: b,
            receiver: r,
        }
    }
    pub async fn read(&mut self) -> Result<Message> {
        Ok(self.receiver.recv().await?)
    }
    pub fn send(&self, msg: Message) -> Result<()> {
        Ok(self.sender.send(msg)?)
    }
}

pub trait ConnectionIO {
    async fn read(&mut self) -> Result<Message>;
    fn send(&self, msg: Message) -> Result<()>;
}

impl ConnectionIO for Connection {
    async fn read(&mut self) -> Result<Message> {
        Connection::read(self).await
    }
    fn send(&self, msg: Message) -> Result<()> {
        Connection::send(self, msg)
    }
}
