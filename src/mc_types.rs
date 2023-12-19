// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::Serialize;

pub const VERSION_NAME: &str = "1.19.4";
pub const VERSION_PROTOCOL: i32 = 762;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[derive(Serialize)]
pub struct Chat {
    pub text: String,
}

pub async fn read_var_int(stream: &mut OwnedReadHalf) -> i32 {
    let mut value: i32 = 0;
    let mut position: u32 = 0;

    loop {
        let current_byte = stream.read_u8()
            .await
            .expect("Error reading from stream");
        value |= ((current_byte & SEGMENT_BITS) as i32) << position;

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }

        position += 7;

        if position >= 32 {
            eprintln!("VarInt is too big");
        }
    }

    value
}

pub async fn write_var_int(stream: &mut OwnedWriteHalf, mut value: i32) {
    loop {
        if (value & !(SEGMENT_BITS as i32)) == 0 {
            stream.write_u8(value as u8)
                .await.expect("Error writing to stream");
            return;
        }

        stream.write_u8((value & (SEGMENT_BITS as i32)) as u8 | CONTINUE_BIT)
            .await
            .expect("Error writing to stream");

        value >>= 7;
    }
}

pub fn write_var_int_bytes(stream: &mut Vec<u8>, mut value: i32) {
    loop {
        if (value & !(SEGMENT_BITS as i32)) == 0 {
            stream.append(&mut vec![value as u8]);
            return;
        }
        stream.append(
            &mut vec![(value & (SEGMENT_BITS as i32)) as u8 | CONTINUE_BIT]);
        value >>= 7;
    }
}

pub async fn read_var_long(stream: &mut OwnedReadHalf) -> i64 {
    let mut value: i64 = 0;
    let mut position: u32 = 0;

    loop {
        let current_byte = stream.read_u8()
            .await
            .expect("Error reading from stream");
        value |= ((current_byte & SEGMENT_BITS) as i64) << position;

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }

        position += 7;

        if position >= 64 {
            eprintln!("VarLong is too big");
        }
    }

    value
}

pub async fn write_var_long(stream: &mut OwnedWriteHalf, mut value: i64) {
    loop {
        if (value & !(SEGMENT_BITS as i64)) == 0 {
            stream.write_u8(value as u8)
                .await
                .expect("Error writing to stream");
            return;
        }

        stream.write_u8((value & SEGMENT_BITS as i64) as u8 | CONTINUE_BIT)
            .await
            .expect("Error writing to stream");

        value >>= 7;
    }
}

pub async fn read_string(stream: &mut OwnedReadHalf) -> String {
    let length = read_var_int(stream).await;
    let mut buffer = vec![0; length as usize];
    stream.read_exact(&mut buffer)
        .await.expect("Error reading string from stream");
    String::from_utf8_lossy(&buffer).to_string()
}

pub async fn write_string(stream: &mut OwnedWriteHalf, s: &str) {
    let length = s.len() as i32;
    write_var_int(stream, length).await;
    stream.write_all(s.as_bytes())
        .await.expect("Error writing string to stream");
}

pub fn write_string_bytes(stream: &mut Vec<u8>, s: &str) {
    let length = s.len() as i32;
    write_var_int_bytes(stream, length);
    stream.append(&mut s.as_bytes().to_vec());
}
