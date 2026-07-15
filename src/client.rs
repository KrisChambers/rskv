use std::error::Error;
use std::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::common::{Frame, FrameType, KVError};

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
                let command = frame.command.clone();
                let msg = frame.serialize();

                if command == FrameType::Sub {
                    self.state = ClientState::Subscribed;
                }

                self.stream.write_all(msg.as_bytes()).await?;
                println!("Getting response");
                self.read_response().await?;
                self.stdin_buffer.clear();

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

    fn read_frame<'a>(&'a mut self) -> KVError<Frame<'a>> {
        self.stdin.read_line(&mut self.stdin_buffer)?;

        let frame: Frame = Frame::deserialize(&self.stdin_buffer);

        Ok(frame)
    }

    async fn read_response(&mut self) -> KVError<&str> {
        let _ = self.stream.read(&mut self.stream_buffer).await?;
        let resp = str::from_utf8(&self.stream_buffer)?;
        println!("<<<: {resp}");
        Ok(str::from_utf8(&self.stream_buffer)?.trim())
    }
}
