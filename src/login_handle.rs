// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use purple_cello_mc_protocol::{
    mc_types::{self, Result, Packet},
    handshake,
    login,
};

struct Player {
    name: String,
    player_uuid: Option<u128>,
}

fn check_player(player: &Player) -> Result<bool> {
    Ok(true)
}

pub async fn respond_login(
    client_reader: &mut OwnedReadHalf,
    client_writer: &mut OwnedWriteHalf,
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) -> Result<bool> {
    let proxy_login = login_to_proxy(client_reader).await?;
    match proxy_login {
        Some(player) => {
            login_to_backend(
                player,
                client_writer,
                server_reader,
                server_writer,
            ).await?;
            return Ok(true)
        },
        None => return Ok(false)
    }
}

async fn login_to_proxy(
    client_reader: &mut OwnedReadHalf,
) -> Result<Option<Player>> {
    println!("Logging into proxy");

    let start_packet =
        login::serverbound::LoginStart::read(client_reader).await?;

    let player: Player = Player {
        name: start_packet.name,
        player_uuid: start_packet.player_uuid,
    };

    if check_player(&player)? {
        println!("Player allowed");
        Ok(Some(player))
    } else {
        println!("Player blocked");
        Ok(None)
    }
}

async fn login_to_backend(
    player: Player,
    client_writer: &mut OwnedWriteHalf,
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
) -> Result<()> {
    println!("Logging into backend");
    handshake::serverbound::Handshake {
        protocol_version: mc_types::VERSION_PROTOCOL,
        server_address: "localhost".to_string(),
        server_port: 25565,
        next_state: 2,
    }.write(server_writer).await?;

    println!("Login start");
    login::serverbound::LoginStart {
        name: player.name,
        player_uuid: player.player_uuid,
    }.write(server_writer).await?;

    println!("Finishing backend login");
    let packet = login::clientbound::LoginSuccess::read(server_reader).await?;

    println!("Finishing proxy login");
    login::clientbound::LoginSuccess {
        uuid: packet.uuid.clone(),
        username: packet.username.clone(),
        properties: packet.properties.clone(),
    }.write(client_writer).await?;

    println!("Client logged in");

    Ok(())
}
