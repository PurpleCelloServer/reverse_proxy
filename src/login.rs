// Yeahbut December 2023

pub mod clientbound {

    use tokio::net::tcp::OwnedReadHalf;

    use crate::mc_types::{self, Result, Packet, PacketError};

    pub enum Login {
        Disconnect(Disconnect),
    }

    impl Login {
        pub async fn read(stream: &mut OwnedReadHalf) -> Result<Self> {
            let mut data = mc_types::read_data(stream).await?;
            let packet_id = mc_types::get_var_int(&mut data)?;
            if packet_id == Disconnect::packet_id() {
                return Ok(Self::Disconnect(Disconnect::get(&mut data)?))
            } else {
                return Err(Box::new(PacketError::InvalidPacketId))
            }
        }
    }

    pub struct Disconnect {
        pub reason: String
    }

    impl Packet for Disconnect {

        fn packet_id() -> i32 {0}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                reason: mc_types::get_string(&mut data)?
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.reason));

            data
        }

    }

}
