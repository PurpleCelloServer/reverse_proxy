// Yeahbut December 2023

use std::fs::{self, File};
use std::io::Read;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::AsyncWriteExt;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;

use crate::mc_types::{self, Result};
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
) -> Result<StatusPlayers> {
    Ok(get_upstream_status(server_reader, server_writer).await?.players)
}

fn motd() -> String {
    let default = "A Minecraft Server Proxy".to_string();
    let file_path = "./motd.json";

    let data = match fs::read_to_string(file_path) {
        Ok(data) => data,
        Err(_) => return default,
    };

    let motd_data: Value = match serde_json::from_str(&data) {
        Ok(value) => value,
        Err(_) => return default,
    };

    let length1 = motd_data["line1"].as_array().map_or(0, |v| v.len());
    let length2 = motd_data["line2"].as_array().map_or(0, |v| v.len());

    if length1 == 0 || length2 == 0 {
        return default;
    }

    let mut rng = rand::thread_rng();
    let rand1 = rng.gen_range(0..length1) as usize;
    let rand2 = rng.gen_range(0..length2) as usize;

    let line1: &str = match motd_data["line1"][rand1].as_str() {
        Some(s) => s,
        None => return default,
    };

    // TODO: Birthdays, Holidays, and Announcements

    let line2: &str = match motd_data["line2"][rand2].as_str() {
        Some(s) => s,
        None => return default,
    };

    let line: String = format!("{}\n{}", line1, line2);
    line
}

fn favicon() -> Option<String> {
    let file_path = "./main_icon.png";

    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(_) => return None,
    };

    let mut buffer = Vec::new();
    if let Err(_) = file.read_to_end(&mut buffer) {
        return None
    };

    let base64_string = general_purpose::STANDARD_NO_PAD.encode(buffer);
    let full_string: String =
        format!("data:image/png;base64,{}", base64_string);

    Some(full_string)
}

pub async fn respond_status(
    client_reader: &mut OwnedReadHalf,
    client_writer: &mut OwnedWriteHalf,
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
)-> Result<()> {
    loop {
        println!("Status Handling");
        let mut data = mc_types::read_packet(client_reader).await?;
        let packet_id = mc_types::get_var_int(&mut data)?;

        println!("Status Packet ID: {}", packet_id);

        if packet_id == 0x00 {
            println!("Handling Status");
            let online_players =
                online_players(server_reader, server_writer).await?;
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

            let json = serde_json::to_string(&status_response)?;

            let mut out_data: Vec<u8> = vec![0];
            out_data.append(&mut mc_types::convert_string(&json));
            mc_types::write_packet(client_writer, &mut out_data).await?;
        } else if packet_id == 0x01 {
            println!("Handling Ping");
            let mut out_data: Vec<u8> = vec![1];
            out_data.append(&mut data);
            mc_types::write_packet(client_writer, &mut out_data).await?;
            break;
        } else {
            break;
        }
    }
    Ok(())
}

pub async fn get_upstream_status(
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) -> Result<StatusResponseData> {
    handshake::write_handshake(server_writer, handshake::Handshake{
        protocol_version: mc_types::VERSION_PROTOCOL,
        server_address: "localhost".to_string(),
        server_port: 25565,
        next_state: 1,
    }).await?;
    mc_types::write_packet(server_writer, &mut vec![0]).await?;
    let mut data = mc_types::read_packet(server_reader).await?;

    mc_types::get_u8(&mut data);
    let json = mc_types::get_string(&mut data)?;
    let status_response: StatusResponseData = serde_json::from_str(&json)?;

    // let mut out_data: Vec<u8> = vec![1];
    // out_data.append(&mut mc_types::convert_i64(0));
    // mc_types::write_packet(server_writer, &mut out_data).await?;

    Ok(status_response)
}

pub async fn respond_legacy_status(
    client_writer: &mut OwnedWriteHalf,
) -> Result<()> {
    println!("Old Style Status");
    client_writer.write_u8(0xFF).await?;

    let s = "§1\0127\0".to_string() +
        mc_types::VERSION_NAME +
        "\0YTD Proxy§0§10";
    let utf16_vec: Vec<u16> = s
        .encode_utf16()
        .flat_map(|c| std::iter::once(c).chain(std::iter::once(0)))
        .collect();

    client_writer.write_u16((utf16_vec.len() / 2) as u16).await?;
    for utf16_char in utf16_vec {
        client_writer.write_u16(utf16_char).await?;
    }

    Ok(())
}
