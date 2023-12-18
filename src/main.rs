// Yeahbut December 2023

use tokio::net::{TcpListener, TcpStream};
use tokio::io;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:25565").await?;
    println!("Proxy listening on port 25565...");

    while let Ok((client_socket, _)) = listener.accept().await {
        tokio::spawn(handle_client(client_socket));
    }

    Ok(())
}

async fn handle_client(client_socket: TcpStream) {
    let backend_addr = "127.0.0.1:25566";

    if let Ok(mut backend_socket) = TcpStream::connect(backend_addr).await {
        let (mut client_reader, mut client_writer) = client_socket.into_split();
        let (mut backend_reader, mut backend_writer) = backend_socket.into_split();

        // Forward from client to backend
        tokio::spawn(async move {
            if let Err(e) = io::copy(&mut client_reader, &mut backend_writer).await {
                eprintln!("Error copying from client to backend: {}", e);
            }
        });

        // Forward from backend to client
        tokio::spawn(async move {
            if let Err(e) = io::copy(&mut backend_reader, &mut client_writer).await {
                eprintln!("Error copying from backend to client: {}", e);
            }
        });
    } else {
        eprintln!("Failed to connect to the backend server");
    }
}
