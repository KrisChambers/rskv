mod client;
mod common;
mod server;
mod storage;

use std::env;
use std::error::Error;

use client::run_client;
use server::run_server;

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
    use OperationMode::*;

    match get_mode() {
        Ok(Server) => run_server().await,
        Ok(Client) => run_client().await,
        Err(msg) => panic!("{msg}"),
    }
}
