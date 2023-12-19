// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::mc_types;

pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: i32,
}

pub async fn read_handshake(stream: &mut OwnedReadHalf) -> Handshake {
    let mut data = mc_types::read_packet(stream).await;
    let _packet_id = mc_types::get_var_int(&mut data);
    get_handshake(&mut data)
}

pub fn get_handshake(data: &mut Vec<u8>) -> Handshake {
    Handshake {
        protocol_version: mc_types::get_var_int(data),
        server_address: mc_types::get_string(data),
        server_port: mc_types::get_u16(data),
        next_state: mc_types::get_var_int(data),
    }
}

pub fn convert_handshake(handshake: Handshake) -> Vec<u8> {
    let mut data: Vec<u8> = vec![0];
    data.append(&mut mc_types::convert_var_int(handshake.protocol_version));
    data.append(&mut mc_types::convert_string(&handshake.server_address));
    data.append(&mut mc_types::convert_u16(handshake.server_port));
    data.append(&mut mc_types::convert_var_int(handshake.next_state));

    data
}

pub async fn write_handshake(
    stream: &mut OwnedWriteHalf,
    handshake: Handshake,
) {
    mc_types::write_packet(stream, &mut convert_handshake(handshake)).await;
}
