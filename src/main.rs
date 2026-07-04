use std::error::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream},
};

fn get_addr() -> String {
    "0.0.0.0:6666".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = get_addr();

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening at {addr}");

    loop {
        let (mut socket, _) = listener.accept().await?;

        println!("Accepting connection");

        tokio::spawn(async move {
            echo(&mut socket).await.expect("Failed to echo request")
        });
    }
}

async fn echo(socket: &mut TcpStream) -> Result<(), Box<dyn Error>>{
    let mut buf = vec![0; 1024];

    loop {
        let n = socket.read(&mut buf).await?;
        println!("Read {n} bytes");

        if n == 0 { return Ok(()); };

        socket.write_all(&buf[0..n]).await?;
    }
}
