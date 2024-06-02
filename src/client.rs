// Yeahbut May 2024

use std::mem;
use tokio::net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}};

use purple_cello_mc_protocol::{
    mc_types::{self, Packet, ProtocolConnection},
    handshake,
    login,
};

use crate::status_handle;
use crate::login_handle;
use crate::listener;

pub async fn handle_client(
    client_socket: TcpStream,
    proxy_info: listener::ProxyInfo,
) {
    println!("Accepting Connection");
    let backend_addr = proxy_info.formatted_backend_address();

    let (mut client_reader, mut client_writer) = client_socket.into_split();
    let mut client_conn = ProtocolConnection::new(
        &mut client_reader,
        &mut client_writer,
    );

    let mut backend_socket: (OwnedReadHalf, OwnedWriteHalf);
    let mut server_conn: Option<ProtocolConnection<'_>> =
        match TcpStream::connect(backend_addr).await {
            Ok(backend_stream) => {
                backend_socket = backend_stream.into_split();
                Some(ProtocolConnection::new(
                    &mut backend_socket.0, &mut backend_socket.1))
            },
            Err(_) => None,
    };

    let mut buffer: [u8; 1] = [0; 1];
    client_conn.stream_read.peek(&mut buffer)
        .await.expect("Failed to peek at first byte from stream");
    let packet_id: u8 = buffer[0];

    if packet_id == 0xFE {
        status_handle::respond_legacy_status(&mut client_conn)
            .await.expect("Error handling legacy status request");
        return;
    } else {
        let handshake_packet =
            handshake::serverbound::Handshake::read(&mut client_conn)
                .await.expect("Error reading handshake packet");
        println!("Next state: {}", handshake_packet.next_state);
        if handshake_packet.next_state == 1 {
            println!("Receiving Status Request");
            status_handle::respond_status(
                proxy_info,
                &mut client_conn,
                &mut server_conn,
            ).await.expect("Error handling status request");
            return;
        } else if handshake_packet.next_state == 2 {
            if handshake_packet.protocol_version == mc_types::VERSION_PROTOCOL {
                match server_conn {
                    Some(mut server_conn) => {
                        if login_handle::respond_login(
                            &proxy_info,
                            &mut client_conn,
                            &mut server_conn,
                        ).await.expect(
                            "Error logging into proxy or server"
                        ) {
                            handle_play(
                                client_conn,
                                server_conn,
                            ).await;
                        } else {
                            println!("Player blocked from server");
                        }
                    }
                    None => {
                        login::clientbound::Disconnect {
                            reason: "\"Server Error (Server is down or \
restarting)\nPlease contact the admins if the issue persists:\n\
purplecelloserver@gmail.com\""
                                .to_string()
                        }
                            .write(&mut client_conn)
                            .await
                            .expect("Error sending disconnect on: \
Failed to connect to the backend server");
                        return;
                    }
                };
            }
            else
            if handshake_packet.protocol_version < mc_types::VERSION_PROTOCOL {
                println!("Client on outdated version");
                login::clientbound::Disconnect {
                    reason: format!(
                        "\"Client Error: Outdated Version (I'm on {})\"",
                        mc_types::VERSION_NAME,
                    ).to_string()
                }
                    .write(&mut client_conn).await.expect(
                        "Error sending disconnect on: Client on wrong version");
            // if handshake_packet.protocol_version > mc_types::VERSION_PROTOCOL
            } else {
                println!("Client on future version");
                login::clientbound::Disconnect {
                    reason: format!(
                        "\"Client Error: Future Version (I'm on {})\"",
                        mc_types::VERSION_NAME,
                    ).to_string()
                }
                    .write(&mut client_conn).await.expect(
                        "Error sending disconnect on: Client on wrong version");
            }
        } else {
            return;
        }
    }


    println!("Connection Closed");
}

async fn handle_play<'a>(
    mut client_conn: ProtocolConnection<'a>,
    mut server_conn: ProtocolConnection<'a>,
) {
    let client_conn: &mut ProtocolConnection<'static> =
        unsafe { mem::transmute(&mut client_conn) };
    let server_conn: &mut ProtocolConnection<'static> =
        unsafe { mem::transmute(&mut server_conn) };

    let (mut client_write_conn, mut client_read_conn) =
        client_conn.split_conn().expect(
            "Error copying from client to backend");
    let (mut server_write_conn, mut server_read_conn) =
        server_conn.split_conn().expect(
            "Error copying from backend to client");

    // Forward from client to backend
    let to_backend = tokio::spawn(async move {
        client_read_conn.forward_play(&mut server_write_conn).await.expect(
            "Error copying from client to backend");
    });

    // Forward from backend to client
    let to_client = tokio::spawn(async move {
        server_read_conn.forward_play(&mut client_write_conn).await.expect(
            "Error copying from backend to client");
    });

    tokio::try_join!(to_backend, to_client).expect(
        "Error copying between the client and backend");
}
