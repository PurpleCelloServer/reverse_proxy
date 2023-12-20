// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::mc_types::{self, Result};

pub fn convert_clientbound_disconnect(reason: String) -> Vec<u8> {
    let mut data: Vec<u8> = vec![0];
    data.append(&mut &mut mc_types::convert_string(&reason));

    data
}
pub async fn write_clientbound_disconnect(
    stream: &mut OwnedWriteHalf,
    reason: String,
) -> Result<()> {
    mc_types::write_packet(
        stream,
        &mut convert_clientbound_disconnect(reason),
    ).await?;
    Ok(())
}
