// Yeahbut December 2023

pub mod clientbound {

    use tokio::net::tcp::OwnedReadHalf;

    use crate::mc_types::{self, Result, Packet, PacketArray, PacketError};

    pub enum Login {
        Disconnect(Disconnect),
        EncryptionRequest(EncryptionRequest),
        LoginSuccess(LoginSuccess),
    }

    impl Login {
        pub async fn read(stream: &mut OwnedReadHalf) -> Result<Self> {
            let mut data = mc_types::read_data(stream).await?;
            let packet_id = mc_types::get_var_int(&mut data)?;
            if packet_id == Disconnect::packet_id() {
                return Ok(Self::Disconnect(Disconnect::get(&mut data)?))
            } else if packet_id == EncryptionRequest::packet_id() {
                return Ok(Self::EncryptionRequest(
                    EncryptionRequest::get(&mut data)?))
            } else if packet_id == LoginSuccess::packet_id() {
                return Ok(Self::LoginSuccess(LoginSuccess::get(&mut data)?))
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

    pub struct EncryptionRequest {
        pub server_id: String,
        pub public_key: Vec<u8>,
        pub verify_token: Vec<u8>,
    }

    impl Packet for EncryptionRequest {

        fn packet_id() -> i32 {1}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                server_id: mc_types::get_string(&mut data)?,
                public_key: mc_types::get_byte_array(&mut data)?,
                verify_token: mc_types::get_byte_array(&mut data)?,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_string(&self.server_id));
            data.append(&mut mc_types::convert_byte_array(
                &mut self.public_key.clone()));
            data.append(&mut mc_types::convert_byte_array(
                &mut self.verify_token.clone()));

            data
        }

    }

    pub struct LoginSuccess {
        pub uuid: u128,
        pub username: String,
        pub properties: Vec<LoginSuccessProperty>,
    }

    impl Packet for LoginSuccess {

        fn packet_id() -> i32 {2}

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            Ok(Self {
                uuid: mc_types::get_uuid(&mut data),
                username: mc_types::get_string(&mut data)?,
                properties: LoginSuccessProperty::get_array(&mut data)?,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_var_int(Self::packet_id()));
            data.append(&mut mc_types::convert_uuid(self.uuid));
            data.append(&mut mc_types::convert_string(&self.username));
            data.append(&mut LoginSuccessProperty::convert_array(
                &mut self.properties.clone()));

            data
        }

    }

    pub struct LoginSuccessProperty {
        name: String,
        value: String,
        signature: Option<String>,
    }

    impl Clone for LoginSuccessProperty {
        fn clone(&self) -> Self {
            Self {
                name: self.name.clone(),
                value: self.value.clone(),
                signature: self.signature.clone(),
            }
        }
    }

    impl PacketArray for LoginSuccessProperty {

        fn get(mut data: &mut Vec<u8>) -> Result<Self> {
            let name = mc_types::get_string(&mut data)?;
            let value = mc_types::get_string(&mut data)?;
            let is_signed = mc_types::get_bool(&mut data);
            let mut signature: Option<String> = None;
            if is_signed {
                signature = Some(mc_types::get_string(&mut data)?);
            }
            Ok(Self {
                name,
                value,
                signature,
            })
        }

        fn convert(&self) -> Vec<u8> {
            let mut data: Vec<u8> = vec![];
            data.append(&mut mc_types::convert_string(&self.name));
            data.append(&mut mc_types::convert_string(&self.value));
            match &self.signature {
                Some(value) => {
                    data.append(&mut &mut mc_types::convert_bool(true));
                    data.append(&mut mc_types::convert_string(&value));
                },
                None => data.append(&mut &mut mc_types::convert_bool(false))
            }

            data
        }

    }

}
