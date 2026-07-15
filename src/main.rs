mod client;
mod common;
mod server;
mod storage;

use std::env;
use std::error::Error;

use client::{Client, ClientState};
use server::{Server, ServerState};

const SERVER_ADDR: &str = "0.0.0.0:6666";

enum OperationMode {
    Server,
    Client,
}

fn get_mode() -> Result<OperationMode, String> {
    let value = env::args().skip(1).take(1).collect::<Vec<String>>();

    if value.is_empty() {
        Err("No Operation Mode detected".into())
    } else {
        match value[0].as_str() {
            "c" => Ok(OperationMode::Client),
            "s" => Ok(OperationMode::Server),
            other => Err(format!("Invalid Mode: {other}")),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match get_mode() {
        Ok(OperationMode::Server) => {
            let mut server = Server::new(SERVER_ADDR).await?;

            while server.state == ServerState::Active {
                server.poll().await?;
            }

            Ok(())
        },
        Ok(OperationMode::Client) => {
            let mut client = Client::new(SERVER_ADDR).await?;

            while client.state != ClientState::Closed {
                client.poll().await?;
            }

            Ok(())
        },
        Err(msg) => panic!("{msg}"),
    }
}
