// Yeahbut December 2023

use tokio::net::{TcpListener, TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}};
use tokio::io;
use std::error::Error;

use purple_cello_mc_protocol::{
    mc_types::{self, Packet},
    handshake,
    login,
};

mod status_handle;


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

    // "Failed to connect to the backend server"

    let backend_socket = match TcpStream::connect(backend_addr).await {
        Ok(backend_socket) => Some(backend_socket.into_split()),
        Err(_) => None,
    };

    let (mut server_reader, mut server_writer):
        (Option<OwnedReadHalf>, Option<OwnedWriteHalf>) =
            match backend_socket {
                Some(backend_socket) =>
                    (Some(backend_socket.0), Some(backend_socket.1)),
                None => (None, None),
    };

    let mut buffer: [u8; 1] = [0; 1];
    client_reader.peek(&mut buffer)
        .await.expect("Failed to peek at first byte from stream");
    let packet_id: u8 = buffer[0];

    if packet_id == 0xFE {
        status_handle::respond_legacy_status(&mut client_writer)
            .await.expect("Error handling legacy status request");
        return;
    } else {
        let handshake_packet =
            handshake::serverbound::Handshake::read(&mut client_reader)
                .await.expect("Error reading handshake packet");
        println!("Next state: {}", handshake_packet.next_state);
        if handshake_packet.next_state == 1 {
            println!("Receiving Status Request");
            status_handle::respond_status(
                &mut client_reader,
                &mut client_writer,
                &mut server_reader,
                &mut server_writer,
            ).await.expect("Error handling status request");
            return;
        } else if handshake_packet.next_state == 2 {
            match server_writer {
                Some(mut server_writer) => {
                    handshake::serverbound::Handshake {
                        protocol_version: mc_types::VERSION_PROTOCOL,
                        server_address: "localhost".to_string(),
                        server_port: 25565,
                        next_state: 2,
                    }
                        .write(&mut server_writer)
                        .await
                        .expect("Error logging into backend server");

                    // Forward from client to backend
                    tokio::spawn(async move {
                        io::copy(&mut client_reader, &mut server_writer).await
                            .expect("Error copying from client to backend");
                    });

                    // Forward from backend to client
                    match server_reader {
                        Some(mut server_reader) => tokio::spawn(async move {
                            io::copy(&mut server_reader, &mut client_writer)
                                .await
                                .expect("Error copying from backend to client");
                        }),
                        None => {
                            eprintln!(
                                "Failed to connect to the backend server");
                            return;
                        }
                    };
                },
                None => {
                    login::clientbound::Disconnect {
                        reason: "\"Server Error (Server may be starting)\""
                            .to_string()
                    }
                        .write(&mut client_writer)
                        .await
                        .expect("Error sending disconnect on: \
Failed to connect to the backend server");
                }
            };
        } else {
            return;
        }
    }


    println!("Connection Closed");
}
