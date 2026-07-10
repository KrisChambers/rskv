use crate::common::{Frame, KVError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, channel};
use tokio::task::JoinHandle;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

type Storage = Arc<RwLock<HashMap<String, String>>>;
type ChannelMap = Arc<RwLock<HashMap<String, Sender<String>>>>;

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
            store: Arc::new(RwLock::new(HashMap::new())),
            channel_map: Arc::new(RwLock::new(HashMap::new())),
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
    let mut buffer = vec![0; 1024];

    let writer = Arc::new(RwLock::new(writer));

    loop {
        let n = buf_reader.read(&mut buffer).await?;
        println!(":::: Read {n} bytes");

        if n == 0 {
            return Ok(());
        };

        let frame: Frame = bincode::deserialize(&buffer).unwrap();
        println!("<<<: {frame:?}");

        match frame {
            Frame::Get(name) => {
                let store = storage.read().await;

                let value = store.get(&name).ok_or(format!("Invalid key {name}"))?;
                writer.write().await.write_all(value.as_bytes()).await?;
            }
            Frame::Set(name, value) => {
                let mut store = storage.write().await;
                let _ = store.insert(name.clone(), value.clone());

                writer
                    .write()
                    .await
                    .write_all(format!("{name} -> {value}").as_bytes())
                    .await?
            }
            Frame::Pub(name, msg) => {
                let channel_store = channels.read().await;

                let channel = channel_store
                    .get(&name)
                    .ok_or(format!("no channel for '{name}'"))?;
                println!("<< {name} << {msg}");

                channel.send(msg).await?;

                writer.write().await.write_all("Sent".as_bytes()).await?
            }
            Frame::Sub(name) => {
                let spawn_name = name.clone();
                let spawn_writer = writer.clone();
                println!("::: Added Channel for {name}");

                let mut channel_map = channels.write().await;

                if !channel_map.contains_key(&name) {
                    let (sender, mut receiver) = channel::<String>(100);

                    channel_map.insert(name.trim().to_string(), sender);

                    tokio::spawn(async move {
                        loop {
                            println!("Waiting for receiver");
                            if let Some(msg) = receiver.recv().await {
                                println!("<<< {msg}");
                                spawn_writer
                                    .write()
                                    .await
                                    .write_all(format!("{spawn_name} -> {msg}").as_bytes())
                                    .await
                                    .unwrap();

                                println!("::: Sent message on {spawn_name}");
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
