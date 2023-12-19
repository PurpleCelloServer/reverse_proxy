// Yeahbut December 2023

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};

pub const VERSION_NAME: &str = "1.19.4";
pub const VERSION_PROTOCOL: i32 = 762;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[derive(Serialize, Deserialize)]
pub struct Chat {
    pub text: String,
}

pub async fn read_packet(stream: &mut OwnedReadHalf) -> Vec<u8> {
    let length = read_var_int_stream(stream).await;
    let mut buffer: Vec<u8> = vec![0; length as usize];
    stream.read_exact(&mut buffer)
        .await.expect("Error reading string from stream");
    buffer
}
pub async fn write_packet(stream: &mut OwnedWriteHalf, data: &mut Vec<u8>) {
    let length = data.len() as i32;
    stream.write_all(&convert_var_int(length))
        .await.expect("Error writing to stream");
    stream.write_all(&data)
        .await.expect("Error writing to stream");
}
async fn read_var_int_stream(stream: &mut OwnedReadHalf) -> i32 {
    let mut data: Vec<u8> = vec![];

    loop {
        let current_byte = stream.read_u8()
            .await.expect("Error reading from stream");

        data.append(&mut vec![current_byte]);

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }
    }

    get_var_int(&mut data)
}

pub fn get_bool(data: &mut Vec<u8>) -> bool {
    data.remove(0) != 0
}
pub fn convert_bool(value: bool) -> Vec<u8> {
    vec![value as u8]
}

pub fn get_u8(data: &mut Vec<u8>) -> u8 {
    data.remove(0)
}
pub fn convert_u8(value: u8) -> Vec<u8> {
    vec![value]
}

pub fn get_i8(data: &mut Vec<u8>) -> i8 {
    get_u8(data) as i8
}
pub fn convert_i8(value: i8) -> Vec<u8> {
    convert_u8(value as u8)
}

pub fn get_u16(data: &mut Vec<u8>) -> u16 {
    ((data.remove(0) as u16) << 8) |
    (data.remove(0) as u16)
}
pub fn convert_u16(value: u16) -> Vec<u8> {
    vec![
        ((value & 0xFF00) >> 8) as u8,
        (value & 0xFF) as u8,
    ]
}

pub fn get_i16(data: &mut Vec<u8>) -> i16 {
    get_u16(data) as i16
}
pub fn convert_i16(value: i16) -> Vec<u8> {
    convert_u16(value as u16)
}

pub fn get_u32(data: &mut Vec<u8>) -> u32 {
    ((data.remove(0) as u32) << 24) |
    ((data.remove(0) as u32) << 16) |
    ((data.remove(0) as u32) << 8) |
    (data.remove(0) as u32)
}
pub fn convert_u32(value: u32) -> Vec<u8> {
    vec![
        ((value & 0xFF0000) >> 24) as u8,
        ((value & 0xFF0000) >> 16) as u8,
        ((value & 0xFF00) >> 8) as u8,
        (value & 0xFF) as u8,
    ]
}

pub fn get_i32(data: &mut Vec<u8>) -> i32 {
    get_u32(data) as i32
}
pub fn convert_i32(value: i32) -> Vec<u8> {
    convert_u32(value as u32)
}

pub fn get_f32(data: &mut Vec<u8>) -> f32 {
    get_u32(data) as f32
}
pub fn convert_f32(value: f32) -> Vec<u8> {
    convert_u32(value as u32)
}

pub fn get_u64(data: &mut Vec<u8>) -> u64 {
    ((data.remove(0) as u64) << 56) |
    ((data.remove(0) as u64) << 48) |
    ((data.remove(0) as u64) << 40) |
    ((data.remove(0) as u64) << 32) |
    ((data.remove(0) as u64) << 24) |
    ((data.remove(0) as u64) << 16) |
    ((data.remove(0) as u64) << 8) |
    (data.remove(0) as u64)
}
pub fn convert_u64(value: u64) -> Vec<u8> {
    vec![
        ((value & 0xFF00000000000000) >> 56) as u8,
        ((value & 0xFF000000000000) >> 48) as u8,
        ((value & 0xFF0000000000) >> 40) as u8,
        ((value & 0xFF00000000) >> 32) as u8,
        ((value & 0xFF000000) >> 24) as u8,
        ((value & 0xFF0000) >> 16) as u8,
        ((value & 0xFF00) >> 8) as u8,
        (value & 0xFF) as u8,
    ]
}

pub fn get_i64(data: &mut Vec<u8>) -> i64 {
    get_u64(data) as i64
}
pub fn convert_i64(value: i64) -> Vec<u8> {
    convert_u64(value as u64)
}

pub fn get_f64(data: &mut Vec<u8>) -> f64 {
    get_u64(data) as f64
}
pub fn convert_f64(value: f64) -> Vec<u8> {
    convert_u64(value as u64)
}

pub fn get_var_int(data: &mut Vec<u8>) -> i32 {
    let mut value: i32 = 0;
    let mut position: u32 = 0;

    loop {
        let current_byte = data.remove(0);
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
pub fn convert_var_int(mut value: i32) -> Vec<u8> {
    let mut data: Vec<u8> = vec![];
    loop {
        if (value & !(SEGMENT_BITS as i32)) == 0 {
            data.append(&mut vec![value as u8]);
            return data;
        }
        data.append(
            &mut vec![(value & (SEGMENT_BITS as i32)) as u8 | CONTINUE_BIT]);
        value >>= 7;
    }
}

pub fn get_string(data: &mut Vec<u8>) -> String {
    let length = get_var_int(data);
    let mut buffer = vec![0; length as usize];
    data.append(&mut buffer);
    String::from_utf8_lossy(&buffer).to_string()
}
pub fn convert_string(s: &str) -> Vec<u8> {
    let length = s.len() as i32;
    let mut data = convert_var_int(length);
    data.append(&mut s.as_bytes().to_vec());
    data
}
