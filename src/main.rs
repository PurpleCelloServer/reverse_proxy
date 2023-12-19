// Yeahbut December 2023

use tokio::net::{TcpListener, TcpStream};
use tokio::io;
use std::error::Error;

mod mc_types;
mod handshake;
mod status;
mod login;

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
    println!("Accepting Connection");
    let backend_addr = "127.0.0.1:25566";

    let (mut client_reader, mut client_writer) = client_socket.into_split();
    if let Ok(backend_socket) = TcpStream::connect(backend_addr).await {
        let (mut server_reader, mut server_writer) =
            backend_socket.into_split();
        let mut buffer: [u8; 1] = [0; 1];
        client_reader.peek(&mut buffer)
            .await.expect("Error reading from stream");
        let packet_id: u8 = buffer[0];

        if packet_id == 0xFE {
            status::respond_legacy_status(&mut client_writer).await;
            return;
        } else {
            let handshake_packet = handshake::read_handshake(&mut client_reader)
                .await;
            println!("Next state: {}", handshake_packet.next_state);
            if handshake_packet.next_state == 1 {
                println!("Receiving Status Request");
                status::respond_status(
                    &mut client_reader,
                    &mut client_writer,
                    &mut server_reader,
                    &mut server_writer,
                ).await;
                return;
            } else if handshake_packet.next_state == 2 {
                handshake::write_handshake(&mut server_writer, handshake::Handshake{
                    protocol_version: mc_types::VERSION_PROTOCOL,
                    server_address: "localhost".to_string(),
                    server_port: 25565,
                    next_state: 2,
                }).await;
            } else {
                return;
            }
        }

        // Forward from client to backend
        tokio::spawn(async move {
            io::copy(&mut client_reader, &mut server_writer)
                .await.expect("Error copying from client to backend");
        });

        // Forward from backend to client
        tokio::spawn(async move {
            io::copy(&mut server_reader, &mut client_writer)
                .await.expect("Error copying from backend to client");
        });
    } else {
        eprintln!("Failed to connect to the backend server");
    }
    println!("Connection Closed");
}
