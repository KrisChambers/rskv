use std::error::Error;
use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::common::{Frame, get_addr};

pub async fn run_client() -> Result<(), Box<dyn Error>> {
    let addr = get_addr();
    let mut socket = TcpStream::connect(&addr).await?;

    // We want to read in command Ex: Get "name"
    let stdin = io::stdin();
    let mut buffer = String::new();
    let mut response = vec![0; 1024];
    let (mut reader, mut writer) = socket.split();

    let mut is_subscribed = false;

    loop {
        if !is_subscribed {
            let _ = stdin.read_line(&mut buffer)?;
            println!(":::: {}",buffer.trim());

            let frame: Frame = (&buffer).into();

            if let Frame::Sub(_) = frame {
                is_subscribed = true;
            }

            let ser_frame = bincode::serialize(&frame)?;
            writer.write_all(&ser_frame).await?;
            println!(">>>: {}", buffer.trim());
        }

        let _ = reader.read(&mut response).await?;
        let resp = str::from_utf8(&response)?.trim();
        println!("<<<: {resp}");
        buffer = String::new();
    }
}
