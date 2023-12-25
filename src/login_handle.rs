// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io;
use std::error::Error;

use purple_cello_mc_protocol::{
    mc_types::{self, Result, Packet},
    handshake,
    login,
};

pub async fn respond_login(
    client_reader: &mut OwnedReadHalf,
    client_writer: &mut OwnedWriteHalf,
    server_reader: &mut OwnedReadHalf,
    server_writer: &mut OwnedWriteHalf,
)-> Result<bool> {
    handshake::serverbound::Handshake {
        protocol_version: mc_types::VERSION_PROTOCOL,
        server_address: "localhost".to_string(),
        server_port: 25565,
        next_state: 2,
    }.write(server_writer).await?;
    Ok(true)
}
