use crate::common::{Frame, get_addr};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc};
use tokio::{
    sync::{RwLock},
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

type Storage = Arc<RwLock<HashMap<String, String>>>;

pub async fn run_server() -> Result<(), Box<dyn Error>> {
    let addr = get_addr();

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening at {addr}");

    let store = Arc::new(RwLock::new(HashMap::new()));

    //let _ = {
    //    let mut s = store.write().await;
    //    s.insert("boop".to_string(), "blop".to_string())

    //};

    loop {
        let (mut stream, _) = listener.accept().await?;

        println!("Accepted connection");

        let store_instance = store.clone();

        tokio::spawn(async move { process(&mut stream, store_instance).await.unwrap() });
    }
}

async fn process(stream: &mut TcpStream, storage: Storage) -> Result<(), Box<dyn Error>> {
    let (reader, mut writer) = stream.split();
    let mut buf_reader = BufReader::new(reader);
    let mut buffer = vec![0;1024];
    loop {
        let n = buf_reader.read( &mut buffer).await?;
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
                writer.write_all(value.as_bytes()).await?;
            }
            Frame::Set(name, value) => {
                let mut store = storage.write().await;
                let _ = store.insert(name.clone(), value.clone());

                writer.write_all(format!("{name} -> {value}").as_bytes()).await?
            },
            Frame::Pub(_, _) => todo!(),
            Frame::Sub(_) => todo!(),
        }
    }
}
