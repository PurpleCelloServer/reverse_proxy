// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};

use crate::mc_types;
use crate::handshake;

#[derive(Serialize, Deserialize)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Serialize, Deserialize)]
pub struct StatusPlayerInfo {
    pub name: String,
    pub id: String,
}

#[derive(Serialize, Deserialize)]
pub struct StatusPlayers {
    pub max: i32,
    pub online: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<Vec<StatusPlayerInfo>>
}

#[derive(Serialize, Deserialize)]
pub struct StatusResponseData {
    pub version: StatusVersion,
    pub description: mc_types::Chat,
    pub players: StatusPlayers,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforcesSecureChat: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previewsChat: Option<bool>,
}

async fn online_players(
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) -> StatusPlayers {
    get_upstream_status(server_reader, server_writer).await.players
}

fn motd() -> String {
    "A Minecraft Server Proxy".to_string()
}

fn favicon() -> Option<String> {
    None
}

pub async fn respond_status(
    client_reader: &mut OwnedReadHalf,
    client_writer: &mut OwnedWriteHalf,
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) {
    loop {
        println!("Status Handling");
        let mut data = mc_types::read_packet(client_reader).await;
        let packet_id = mc_types::get_var_int(&mut data);

        println!("Status Packet ID: {}", packet_id);

        if packet_id == 0x00 {
            println!("Handling Status");
            let online_players = online_players(server_reader, server_writer).await;
            let status_response = StatusResponseData {
                version: StatusVersion {
                    name: mc_types::VERSION_NAME.to_string(),
                    protocol: mc_types::VERSION_PROTOCOL,
                },
                description: mc_types::Chat {
                    text: motd(),
                },
                players: StatusPlayers {
                    max: -13,
                    online: online_players.online,
                    sample: online_players.sample,
                },
                favicon: favicon(),
                enforcesSecureChat: None,
                previewsChat: None,
                // enforcesSecureChat: Some(false),
                // previewsChat: Some(false),
            };

            let json_result = serde_json::to_string(&status_response);

            match json_result {
                Ok(json) => {
                    let mut out_data: Vec<u8> = vec![0];
                    out_data.append(&mut mc_types::convert_string(&json));
                    mc_types::write_packet(client_writer, &mut out_data).await;
                },
                Err(err) => {
                    eprintln!("Error serializing to JSON: {}", err);
                    break;
                },
            }
        } else if packet_id == 0x01 {
            println!("Handling Ping");
            let mut out_data: Vec<u8> = vec![1];
            out_data.append(&mut data);
            mc_types::write_packet(client_writer, &mut out_data).await;
            break;
        } else {
            break;
        }
    }
}

pub async fn get_upstream_status(
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) -> StatusResponseData {
    handshake::write_handshake(server_writer, handshake::Handshake{
        protocol_version: mc_types::VERSION_PROTOCOL,
        server_address: "localhost".to_string(),
        server_port: 25565,
        next_state: 1,
    }).await;
    mc_types::write_packet(server_writer, &mut vec![0]).await;
    let mut data = mc_types::read_packet(server_reader).await;

    mc_types::get_u8(&mut data);
    let json = mc_types::get_string(&mut data);
    let status_response: StatusResponseData = serde_json::from_str(&json)
        .expect("Error parsing JSON");

    // let mut out_data: Vec<u8> = vec![1];
    // out_data.append(&mut mc_types::convert_i64(0));
    // mc_types::write_packet(server_writer, &mut out_data).await;

    status_response
}

pub async fn respond_legacy_status(client_writer: &mut OwnedWriteHalf) {
    println!("Old Style Status");
    client_writer.write_u8(0xFF)
        .await.expect("Error writing to stream");

    // let s = "§1\0127\01.12.2\0YTD Proxy§0§10";
    // println!("String length: {}", s.len());
    // client_writer.write_u16(s.len() as u16)
    //     .await.expect("Error writing to stream");
    // let utf16_bytes: Vec<u16> = s.encode_utf16().collect();
    // for utf16_char in utf16_bytes {
    //     client_writer.write_u16(utf16_char)
    //         .await.expect("Error writing to stream");
    // }

    let s = "§1\0127\0".to_string() +
        mc_types::VERSION_NAME +
        "\0YTD Proxy§0§10";
    let utf16_vec: Vec<u16> = s
        .encode_utf16()
        .flat_map(|c| std::iter::once(c).chain(std::iter::once(0)))
        .collect();
    println!("String length: {}", (utf16_vec.len() / 2));
    client_writer.write_u16((utf16_vec.len() / 2) as u16)
        .await.expect("Error writing to stream");
    for utf16_char in utf16_vec {
        client_writer.write_u16(utf16_char)
            .await.expect("Error writing to stream");
    }

    // let s = b"\x00\xa7\x001\x00\x00\x001\x002\x007\x00\x00\x001\x00.\x001\x0
    //    02\x00.\x002\x00\x00\x00Y\x00T\x00D\x00 \x00P\x00r\x00o\x00x\x00y\x00
    //    \xa7\x000\xa7\x001\x000";
    // println!("String length: {}", s.len());
    // client_writer.write_u16(25)
    //     .await.expect("Error writing to stream");
    // for b in s {
    //     client_writer.write_u8(b)
    //         .await.expect("Error writing to stream");
    // }
}
