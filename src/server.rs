use crate::common::{Frame, FrameType, KVError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, channel};
use tokio::task::JoinHandle;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    //sync::RwLock,
};

use crate::sync::RWLock;

type Storage = Arc<RWLock<HashMap<String, String>>>;
type ChannelMap = Arc<RWLock<HashMap<String, Sender<String>>>>;

#[derive(Eq, PartialEq)]
pub enum ServerState {
    Active,
}

pub struct Server {
    listener: TcpListener,
    store: Storage,
    channel_map: ChannelMap,
    connections: Vec<Connection>,
    pub state: ServerState,
}

impl Server {
    pub async fn new(addr: &str) -> KVError<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
            store: Arc::new(RWLock::new(HashMap::new())),
            channel_map: Arc::new(RWLock::new(HashMap::new())),
            connections: vec![],
            state: ServerState::Active,
        })
    }

    pub async fn poll(&mut self) -> KVError<()> {
        let (stream, _) = self.listener.accept().await?;
        println!("Connection accepted");

        let con_store = self.store.clone();
        let con_channels = self.channel_map.clone();

        self.connections.push(
         Connection::new(stream, con_store, con_channels)
        );

        Ok(())
    }

}

enum ConnectionState {
    Connected,
    Disconnected,
}

struct Connection {
    state: ConnectionState,
    handle: JoinHandle<()>
}

impl Connection {
    pub fn new(stream: TcpStream, storage: Storage, channels: ChannelMap) -> Self {
        let handle =
            tokio::spawn(async move { process(stream, storage, channels).await.unwrap() });

        Self {
            state: ConnectionState::Connected,
            handle
        }
    }
}

async fn process(
    stream: TcpStream,
    storage: Storage,
    channels: ChannelMap,
) -> KVError<()> {
    let (reader, writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);
    let mut buffer = String::new();

    let writer = Arc::new(RWLock::new(writer));

    loop {
        let n = buf_reader.read_to_string(&mut buffer).await?;
        println!(":::: Read {n} bytes");

        if n == 0 {
            return Ok(());
        };

        let frame: Frame = Frame::deserialize(&buffer);
        println!("<<<: {frame:?}");
        let key = frame.key;

        match frame.command {
            FrameType::Get => {
                let store = storage.read();//.await;

                let value = store.get(key).ok_or(format!("Invalid key {key}"))?;
                writer.write().write_all(value.as_bytes()).await?;
            },
            FrameType::Set => {
                let mut store = storage.write();//.await;
                if let Some(value) = frame.value {
                    let _ = store.insert(frame.key.to_string(), value.to_string());
                    writer
                        .write()
                        .write_all(format!("{key} -> {value}").as_bytes())
                        .await?
                } else {
                    panic!("Set command without value");
                }

            }
            FrameType::Pub => {
                let channel_store = channels.read();//.await;

                if let Some(value) = frame.value {
                    let channel = channel_store
                        .get(key)
                        .ok_or(format!("no channel for '{key}'"))?;
                    println!("<< {key} << {value}");

                    channel.send(value.to_string()).await?;

                    writer.write().write_all("Sent".as_bytes()).await?
                }
            }
            FrameType::Sub => {
                let spawn_writer = writer.clone();
                println!("::: Added Channel for {key}");

                let mut channel_map = channels.write();//.await;

                if !channel_map.contains_key(key) {
                    let (sender, mut receiver) = channel::<String>(100);
                    let spawn_key = key.to_string(); // We need to create an owned version here.

                    channel_map.insert(spawn_key.trim().to_string(), sender);

                    tokio::spawn(async move {
                        loop {
                            println!("Waiting for receiver");
                            if let Some(msg) = receiver.recv().await {
                                println!("<<< {msg}");
                                spawn_writer
                                    .write()
                                    .write_all(format!("{spawn_key} -> {msg}").as_bytes())
                                    .await
                                    .unwrap();

                                println!("::: Sent message on {spawn_key}");
                            } else {
                                println!("Sender has been dropped")
                            }
                        }
                    });
                }
            }
        }
    }
}
