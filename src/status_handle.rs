// Yeahbut December 2023

use tokio::io::AsyncWriteExt;

use purple_cello_mc_protocol::{
    mc_types::{self, Result, Packet, ProtocolConnection},
    handshake,
    status,
};

use crate::listener;
use crate::info_messages;
use crate::motd::{motd, favicon};

async fn online_players(
    proxy_info: listener::ProxyInfo,
    server_conn: &mut ProtocolConnection<'_>,
) -> Result<status::clientbound::StatusPlayers> {
    Ok(get_upstream_status(proxy_info, server_conn).await?.players)
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
