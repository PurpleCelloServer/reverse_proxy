// Yeahbut December 2023


use purple_cello_mc_protocol::{
    mc_types::{self, Result, Packet, ProtocolConnection},
    handshake,
    login,
};

use purple_cello_mojang_api::multiplayer_auth;

use crate::listener;
use crate::whitelist::{Player, PlayerAllowed};

async fn check_player(
    proxy_info: &mut listener::ProxyInfo,
    player: Player,
    client_conn: &mut ProtocolConnection<'_>,
) -> Result<PlayerAllowed> {
    match proxy_info.online_status {
        listener::OnlineStatus::Online => {
            let encryption_request = client_conn.create_encryption_request(
                proxy_info.private_key.clone())?;
            encryption_request.write(client_conn).await?;
            let encryption_response =
                login::serverbound::EncryptionResponse::read(
                    client_conn).await?;
            client_conn.handle_encryption_response(encryption_response)?;
            let server_id = client_conn.server_id_hash().await?;
            match proxy_info.authentication_method {
                listener::AuthenticationMethod::Mojang => {
                    match multiplayer_auth::joined(
                        &player.name, &server_id, None).await {
                            Ok(_) => Ok(proxy_info.whitelist
                                .check_player_whitelist(player)
                            ),
                            Err(_) => Ok(PlayerAllowed::False(
                                "Mojang Authentication Failed".to_string()
                            )),
                    }},
                listener::AuthenticationMethod::None =>
                    Ok(proxy_info.whitelist.check_player_whitelist(player))
            }
        },
        listener::OnlineStatus::Offline =>
            Ok(proxy_info.whitelist.check_player_whitelist(player)),
    }
}

pub async fn respond_login(
    proxy_info: &mut listener::ProxyInfo,
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
    proxy_info: &mut listener::ProxyInfo,
    client_conn: &mut ProtocolConnection<'_>,
) -> Result<PlayerAllowed> {
    println!("Logging into proxy");

    let start_packet =
        login::serverbound::LoginStart::read(client_conn).await?;

    let player: Player = Player {
        name: start_packet.name,
        player_uuid: start_packet.player_uuid,
        active: true,
    };

    check_player(proxy_info, player, client_conn).await
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
