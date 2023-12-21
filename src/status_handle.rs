// Yeahbut December 2023

use std::fs::{self, File};
use std::io::Read;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::AsyncWriteExt;
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;

use crate::mc_types::{self, Result, Packet};
use crate::status;
use crate::handshake;

async fn online_players(
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) -> Result<status::clientbound::StatusPlayers> {
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
    server_reader: &mut Option<OwnedReadHalf>,
    server_writer: &mut Option<OwnedWriteHalf>,
)-> Result<()> {
    loop {
        println!("Status Handling");
        let packet =
            status::serverbound::StatusPackets::read(client_reader).await?;
        match packet {
            status::serverbound::StatusPackets::Status(_) => {
                println!("Handling Status");
                let favicon = favicon();

                let online_players = match server_reader {
                    Some(server_reader) => match server_writer {
                        Some(server_writer) => match online_players(
                            server_reader,
                            server_writer,
                        ).await {
                            Ok(value) => Some(value),
                            Err(_) => None,
                        },
                        None => None,
                    },
                    None => None,
                };

                let status_response =
                    match online_players {
                        Some(online_players) =>
                            status::clientbound::StatusResponseData {
                                version: status::clientbound::StatusVersion {
                                    name: mc_types::VERSION_NAME.to_string(),
                                    protocol: mc_types::VERSION_PROTOCOL,
                                },
                                description: mc_types::Chat {
                                    text: motd(),
                                },
                                players: status::clientbound::StatusPlayers {
                                    max: -13,
                                    online: online_players.online,
                                    sample: online_players.sample,
                                },
                                favicon: favicon,
                                enforcesSecureChat: Some(false),
                                previewsChat: Some(false),
                        },
                        None => status::clientbound::StatusResponseData {
                            version: status::clientbound::StatusVersion {
                                name: "Old".to_string(),
                                protocol: 0,
                            },
                            description: mc_types::Chat {
                                text: "Server Error (Server may be starting)"
                                    .to_string() + "\nPurple Cello Server",
                            },
                            players: status::clientbound::StatusPlayers {
                                max: 0,
                                online: 0,
                                sample: None,
                            },
                            favicon: favicon,
                            enforcesSecureChat: Some(false),
                            previewsChat: Some(false),
                        },
                };

                let packet =
                    status::clientbound::Status::from_json(status_response)?;
                packet.write(client_writer).await?;
            },
            status::serverbound::StatusPackets::Ping(packet) => {
                println!("Handling Ping");
                let new_packet = status::clientbound::Ping{
                    payload: packet.payload,
                };
                new_packet.write(client_writer).await?;
                break;
            }
        }
    }
    Ok(())
}

pub async fn get_upstream_status(
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) -> Result<status::clientbound::StatusResponseData> {
    handshake::serverbound::Handshake{
        protocol_version: mc_types::VERSION_PROTOCOL,
        server_address: "localhost".to_string(),
        server_port: 25565,
        next_state: 1,
    }.write(server_writer).await?;
    mc_types::write_data(server_writer, &mut vec![0]).await?;
    let mut data = mc_types::read_data(server_reader).await?;

    mc_types::get_u8(&mut data);
    let json = mc_types::get_string(&mut data)?;
    let status_response: status::clientbound::StatusResponseData =
        serde_json::from_str(&json)?;

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
