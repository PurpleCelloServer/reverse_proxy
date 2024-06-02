// Yeahbut December 2023

use std::fs;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use lazy_static::lazy_static;

use purple_cello_mc_protocol::{
    mc_types::{self, Result, Packet, ProtocolConnection},
    handshake,
    login,
};

use crate::listener;

const EXPIRATION_DURATION: Duration = Duration::from_secs(3600);

struct CachedWhitelist {
    whitelist_data: Value,
    timestamp: Instant,
}

#[derive(PartialEq)]
struct Player {
    name: String,
    player_uuid: Option<u128>,
}

enum PlayerAllowed {
    True(Player),
    False(String),
}

fn load_whitelist() -> Value {
    let file_path = "./whitelist.json";

    let data = match fs::read_to_string(file_path) {
        Ok(data) => data,
        Err(_) => return Value::Null,
    };

    let whitelist_data: Value = match serde_json::from_str(&data) {
        Ok(value) => value,
        Err(_) => return Value::Null,
    };

    whitelist_data
}

fn get_whitelist() -> Vec<Player> {
    lazy_static! {
        static ref WHITELIST_CACHE: Arc<Mutex<Option<CachedWhitelist>>> =
            Arc::new(Mutex::new(None));
    }

    let mut cache = WHITELIST_CACHE.lock().unwrap();

    if let Some(cached_whitelist) = cache.as_ref() {
        if cached_whitelist.timestamp.elapsed() >= EXPIRATION_DURATION {
            println!("Refreshing whitelist cache");
            *cache = Some(CachedWhitelist {
                whitelist_data: load_whitelist(),
                timestamp: Instant::now(),
            });
        }
    } else {
        *cache = Some(CachedWhitelist {
            whitelist_data: load_whitelist(),
            timestamp: Instant::now(),
        });
    }

    let whitelist_data = cache.as_ref().unwrap().whitelist_data.clone();

    std::mem::drop(cache);

    if whitelist_data == Value::Null {
        return Vec::new();
    }

    let whitelist_array = match whitelist_data.as_array() {
        Some(whitelist) => whitelist,
        None => { return Vec::new(); }
    };

    let mut whitelist: Vec<Player> = Vec::new();

    for whitelisted_player in whitelist_array {
        let player_map = match whitelisted_player.as_object() {
            Some(whitelist) => whitelist,
            None => { continue; }
        };

        let name = match player_map.get("name") {
            Some(name) => {
                match name.as_str() {
                    Some(name) => name,
                    None => { continue; }
                }
            },
            None => { continue; }
        };

        let player_uuid = match player_map.get("uuid") {
            Some(uuid) => {
                match uuid.as_str() {
                    Some(uuid) => {
                        match u128::from_str_radix(uuid, 16) {
                            Ok(uuid) => uuid,
                            Err(_) => { continue; }
                        }
                    },
                    None => { continue; }
                }
            },
            None => { continue; }
        };

        whitelist.push(Player {
            name: name.to_string(),
            player_uuid: Some(player_uuid),
        })
    }

    whitelist
}

fn check_player_whitelist(player: Player) -> PlayerAllowed {

    if player.player_uuid.is_none() {
        return PlayerAllowed::False("Invalid UUID".to_string());
    }

    let whitelist = get_whitelist();

    let mut invalid_uuid = false;
    let mut invalid_username = false;

    for wl_player in whitelist {
        if wl_player == player {
            return PlayerAllowed::True(player);
        } else if wl_player.name == player.name {
            invalid_uuid = true;
        } else if wl_player.player_uuid == player.player_uuid {
            invalid_username = true;
        }
    }

    if invalid_uuid {
        PlayerAllowed::False("Invalid UUID".to_string())
    } else if invalid_username {
        PlayerAllowed::False(
            "Invalid Username!\nPlease contact the admins to update your \
username:\npurplecelloserver@gmail.com".to_string()
        )
    } else {
        PlayerAllowed::False("Not whitelisted on this server.\n\
Please direct whitelist requests to the admins:\n\
purplecelloserver@gmail.com".to_string())
    }
}

async fn check_player_online(
    proxy_info: &listener::ProxyInfo,
    player: Player,
    client_conn: &mut ProtocolConnection<'_>,
) -> Result<PlayerAllowed> {
    let encryption_request = client_conn.create_encryption_request(
        proxy_info.private_key.clone())?;
    encryption_request.write(client_conn).await?;
    let encryption_response =
        login::serverbound::EncryptionResponse::read(client_conn).await?;
    client_conn.handle_encryption_response(encryption_response)?;
    // TODO: Make authentication verification request
    Ok(check_player_whitelist(player))
}

fn check_player_offline(player: Player) -> Result<PlayerAllowed> {
    Ok(check_player_whitelist(player))
}

pub async fn respond_login(
    proxy_info: &listener::ProxyInfo,
    client_conn: &mut ProtocolConnection<'_>,
    server_conn: &mut ProtocolConnection<'_>,
) -> Result<bool> {
    let proxy_login = login_to_proxy(proxy_info, client_conn).await?;
    match proxy_login {
        PlayerAllowed::True(player) => {
            println!("Player allowed");
            login_to_backend(
                proxy_info,
                player,
                client_conn,
                server_conn,
            ).await?;
            return Ok(true)
        },
        PlayerAllowed::False(msg) => {
            println!("Player blocked: {}", msg);
            login::clientbound::Disconnect {
                reason: format!("{{\"text\":\"{}\"}}", msg.to_string())
            }.write(client_conn).await?;
            return Ok(false)
        }
    }
}

async fn login_to_proxy(
    proxy_info: &listener::ProxyInfo,
    client_conn: &mut ProtocolConnection<'_>,
) -> Result<PlayerAllowed> {
    println!("Logging into proxy");

    let start_packet =
        login::serverbound::LoginStart::read(client_conn).await?;

    let player: Player = Player {
        name: start_packet.name,
        player_uuid: start_packet.player_uuid,
    };

    match proxy_info.online_status {
        listener::OnlineStatus::Online =>
            check_player_online(proxy_info, player, client_conn).await,
        listener::OnlineStatus::Offline =>
            check_player_offline(player),
    }
}

async fn login_to_backend(
    proxy_info: &listener::ProxyInfo,
    player: Player,
    client_conn: &mut ProtocolConnection<'_>,
    server_conn: &mut ProtocolConnection<'_>,
) -> Result<()> {
    println!("Logging into backend");
    handshake::serverbound::Handshake {
        protocol_version: mc_types::VERSION_PROTOCOL,
        server_address: proxy_info.backend_addr.clone(),
        server_port: proxy_info.backend_port,
        next_state: 2,
    }.write(server_conn).await?;

    println!("Login start");
    login::serverbound::LoginStart {
        name: player.name,
        player_uuid: player.player_uuid,
    }.write(server_conn).await?;

    println!("Finishing backend login");
    let packet = login::clientbound::LoginSuccess::read(server_conn).await?;

    println!("Finishing proxy login");
    login::clientbound::LoginSuccess {
        uuid: packet.uuid.clone(),
        username: packet.username.clone(),
        properties: packet.properties.clone(),
    }.write(client_conn).await?;

    println!("Client logged in");

    Ok(())
}
