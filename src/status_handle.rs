// Yeahbut December 2023

use std::fs::{self, File};
use std::io::Read;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

use tokio::io::AsyncWriteExt;
use serde_json::Value;
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use lazy_static::lazy_static;

use purple_cello_mc_protocol::{
    mc_types::{self, Result, Packet, ProtocolConnection},
    handshake,
    status,
};

use crate::listener;
use crate::info_messages;

const EXPIRATION_DURATION: Duration = Duration::from_secs(3600);

struct CachedMotds {
    motd_data: Value,
    timestamp: Instant,
}

async fn online_players(
    proxy_info: listener::ProxyInfo,
    server_conn: &mut ProtocolConnection<'_>,
) -> Result<status::clientbound::StatusPlayers> {
    Ok(get_upstream_status(proxy_info, server_conn).await?.players)
}

fn load_motds() -> Value {
    let file_path = "./motd.json";

    let data = match fs::read_to_string(file_path) {
        Ok(data) => data,
        Err(_) => return Value::Null,
    };

    let motd_data: Value = match serde_json::from_str(&data) {
        Ok(value) => value,
        Err(_) => return Value::Null,
    };

    motd_data
}

fn get_motds() -> Value {
    lazy_static! {
        static ref MOTDS_CACHE: Arc<Mutex<Option<CachedMotds>>> =
            Arc::new(Mutex::new(None));
    }

    let mut cache = MOTDS_CACHE.lock().unwrap();

    if let Some(cached_motds) = cache.as_ref() {
        if cached_motds.timestamp.elapsed() >= EXPIRATION_DURATION {
            println!("Refreshing MOTD cache");
            *cache = Some(CachedMotds {
                motd_data: load_motds(),
                timestamp: Instant::now(),
            });
        }
    } else {
        *cache = Some(CachedMotds {
            motd_data: load_motds(),
            timestamp: Instant::now(),
        });
    }

    let motds = cache.as_ref().unwrap().motd_data.clone();

    std::mem::drop(cache);

    motds
}

fn motd() -> String {
    let default = "A Minecraft Server Proxy".to_string();

    let motd_data = get_motds();

    if motd_data == Value::Null {
        return default;
    }

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
    let file_path = "./icon.png";

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
    proxy_info: listener::ProxyInfo,
    client_conn: &mut ProtocolConnection<'_>,
    server_conn: &mut Option<ProtocolConnection<'_>>,
)-> Result<()> {
    loop {
        println!("Status Handling");
        let packet =
            status::serverbound::StatusPackets::read(client_conn).await?;
        match packet {
            status::serverbound::StatusPackets::Status(_) => {
                println!("Handling Status");
                let favicon = favicon();

                let online_players = match server_conn {
                    Some(server_conn) =>
                        match online_players(
                            proxy_info.clone(),
                            server_conn,
                        ).await {
                            Ok(value) => Some(value),
                            Err(_) => None,
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
                                text: info_messages::BACKEND_DOWN_PING
                                    .to_string(),
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
                packet.write(client_conn).await?;
            },
            status::serverbound::StatusPackets::Ping(packet) => {
                println!("Handling Ping");
                let new_packet = status::clientbound::Ping{
                    payload: packet.payload,
                };
                new_packet.write(client_conn).await?;
                break;
            }
        }
    }
    Ok(())
}

pub async fn get_upstream_status(
    proxy_info: listener::ProxyInfo,
    server_conn: &mut ProtocolConnection<'_>,
) -> Result<status::clientbound::StatusResponseData> {
    handshake::serverbound::Handshake{
        protocol_version: mc_types::VERSION_PROTOCOL,
        server_address: proxy_info.backend_addr,
        server_port: proxy_info.backend_port,
        next_state: 1,
    }.write(server_conn).await?;
    status::serverbound::Status{}.write(server_conn).await?;
    let packet = status::clientbound::Status::read(server_conn).await?;
    let status_response = packet.get_json()?;

    Ok(status_response)
}

pub async fn respond_legacy_status(
    client_conn: &mut ProtocolConnection<'_>,
) -> Result<()> {
    println!("Old Style Status");
    client_conn.stream_write.write_u8(0xFF).await?;

    let s = "§1\0127\0".to_string() +
        mc_types::VERSION_NAME +
        "\0YTD Proxy§0§10";
    let utf16_vec: Vec<u16> = s
        .encode_utf16()
        .flat_map(|c| std::iter::once(c).chain(std::iter::once(0)))
        .collect();

    client_conn.stream_write.write_u16((utf16_vec.len() / 2) as u16).await?;
    for utf16_char in utf16_vec {
        client_conn.stream_write.write_u16(utf16_char).await?;
    }

    Ok(())
}
