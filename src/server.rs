use crate::common::{Frame, get_addr};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

type Storage = Arc<RwLock<HashMap<String, String>>>;
type ChannelMap = Arc<RwLock<HashMap<String, Sender<String>>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr = get_addr();

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening at {addr}");

    let store = Arc::new(RwLock::new(HashMap::new()));
    let channels: ChannelMap = Arc::new(RwLock::new(HashMap::new()));

    loop {
        let (stream, _) = listener.accept().await?;

        println!("Accepted connection");

        let store_instance = store.clone();
        let channel_instance = channels.clone();

        tokio::spawn(async move {
            process(stream, store_instance, channel_instance)
                .await
                .unwrap()
        });
    }
}

async fn process(
    stream: TcpStream,
    storage: Storage,
    channels: ChannelMap,
) -> Result<(), Box<dyn Error>> {
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

                writer
                    .write()
                    .await
                    .write_all("Sent".as_bytes())
                    .await?
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
