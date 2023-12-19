// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::mc_types;

pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

pub async fn read_handshake(stream: &mut OwnedReadHalf) -> Option<Handshake> {
    Some(Handshake {
        protocol_version: mc_types::read_var_int(stream)
            .await,
        server_address: mc_types::read_string(stream)
            .await,
        server_port: stream.read_u16()
            .await.expect("Error reading from stream"),
        next_state: mc_types::read_var_int(stream)
            .await,
    })
}

pub async fn write_handshake(
    stream: &mut OwnedWriteHalf,
    handshake: Handshake,
) {
    let mut data: Vec<u8> = vec![0];
    mc_types::write_var_int_bytes(&mut data, handshake.protocol_version);
    mc_types::write_string_bytes(&mut data, &handshake.server_address);
    data.append(&mut vec![
        ((handshake.server_port & 0xFF00) >> 8) as u8,
        (handshake.server_port & 0xFF) as u8,
    ]);
    mc_types::write_var_int_bytes(&mut data, handshake.next_state);

    mc_types::write_var_int(stream, data.len() as i32).await;
    stream.write_all(&mut data)
        .await.expect("Error writing to stream");
}
