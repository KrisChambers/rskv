use std::error::Error;
use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::common::{Frame, KVError};

#[derive(Eq, PartialEq)]
pub enum ClientState {
    Subscribed,
    Interactive,
    Close,
    Closed,
}

pub struct Client {
    stream: TcpStream,
    stdin: io::Stdin,
    stdin_buffer: String,
    stream_buffer: Vec<u8>,
    pub state: ClientState,
}

impl Client {
    pub async fn new(addr: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Client {
            stream: TcpStream::connect(addr).await?,
            stdin: io::stdin(), // Recommended to not use this for interactive stuff.
            stdin_buffer: String::new(),
            stream_buffer: vec![0; 1024],
            state: ClientState::Interactive,
        })
    }

    pub async fn poll(&mut self) -> KVError<()> {
        match self.state {
            ClientState::Interactive => {
                let frame = self.read_frame()?;

                if let Frame::Sub(_) = frame {
                    self.state = ClientState::Subscribed;
                }

                println!(">>>: {frame:?}");
                self.send_frame(frame).await?;

                self.read_response().await?;

                Ok(())
            }
            ClientState::Subscribed => {
                self.read_response().await?;
                Ok(())
            }
            ClientState::Close => todo!(),
            ClientState::Closed => todo!(),
        }
    }

    fn read_frame(&mut self) -> KVError<Frame> {
        self.stdin.read_line(&mut self.stdin_buffer)?;

        let frame: Frame = (&self.stdin_buffer).into();

        self.stdin_buffer = String::new();

        Ok(frame)
    }

    async fn send_frame(&mut self, frame: Frame) -> KVError<()> {
        let msg = bincode::serialize(&frame)?;
        self.stream.write_all(&msg).await?;

        Ok(())
    }

    async fn read_response(&mut self) -> KVError<&str> {
        let _ = self.stream.read(&mut self.stream_buffer).await?;
        let resp = str::from_utf8(&self.stream_buffer)?;
        println!("<<<: {resp}");
        Ok(str::from_utf8(&self.stream_buffer)?.trim())
    }
}
