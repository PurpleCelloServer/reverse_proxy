// Yeahbut December 2023

use std::error::Error;
use std::fmt;

use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub const VERSION_NAME: &str = "1.19.4";
pub const VERSION_PROTOCOL: i32 = 762;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[derive(Debug)]
pub enum PacketError {
    InvalidPacketId,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketError::InvalidPacketId =>
                write!(f, "Invalid packet id"),
        }
    }
}

impl Error for PacketError {}

#[derive(Debug)]
pub enum VarIntError {
    ValueTooLarge,
    RanOutOfBytes,
}

impl fmt::Display for VarIntError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarIntError::ValueTooLarge =>
                write!(f, "VarInt value is too large"),
            VarIntError::RanOutOfBytes =>
                write!(f, "Ran out of bytes while reading VarInt"),
        }
    }
}

impl Error for VarIntError {}

#[derive(Serialize, Deserialize)]
pub struct Chat {
    pub text: String,
}

#[async_trait]
pub trait Packet: Sized {
    fn packet_id() -> i32;
    fn get(data: &mut Vec<u8>) -> Result<Self>;
    fn convert(&self) -> Vec<u8>;

    async fn read(stream: &mut OwnedReadHalf) -> Result<Self> {
        let mut data = read_data(stream).await?;
        let packet_id = get_var_int(&mut data)?;
        if packet_id == Self::packet_id() {
            return Ok(Self::get(&mut data)?)
        } else {
            return Err(Box::new(PacketError::InvalidPacketId))
        }
    }

    async fn write(&self, stream: &mut OwnedWriteHalf) -> Result<()> {
        write_data(stream, &mut self.convert()).await
    }
}

pub async fn read_data(stream: &mut OwnedReadHalf) -> Result<Vec<u8>> {
    let length = read_var_int_stream(stream).await? as usize;

    let mut buffer: Vec<u8> = vec![0; length];
    stream.read_exact(&mut buffer).await?;

    Ok(buffer)
}
pub async fn write_data(
    stream: &mut OwnedWriteHalf,
    data: &mut Vec<u8>,
) -> Result<()> {
    let mut out_data = convert_var_int(data.len() as i32);
    out_data.append(data);

    stream.write_all(&out_data).await?;

    Ok(())
}
async fn read_var_int_stream(stream: &mut OwnedReadHalf) -> Result<i32> {
    let mut data: Vec<u8> = vec![];

    loop {
        let current_byte = stream.read_u8().await?;

        data.append(&mut vec![current_byte]);

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }
    }

    let varint = get_var_int(&mut data)?;

    Ok(varint)
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

pub fn get_var_int(data: &mut Vec<u8>) -> Result<i32> {
    Ok(get_var(data, 32)? as i32)
}
pub fn convert_var_int(value: i32) -> Vec<u8> {
    convert_var(value as i64)
}

pub fn get_var_long(data: &mut Vec<u8>) -> Result<i64> {
    get_var(data, 64)
}
pub fn convert_var_long(value: i64) -> Vec<u8> {
    convert_var(value)
}

fn get_var(data: &mut Vec<u8>, size: u8) -> Result<i64> {
    let mut value: i64 = 0;
    let mut position: u8 = 0;

    loop {
        if data.is_empty() {
            return Err(Box::new(VarIntError::RanOutOfBytes));
        }

        let current_byte = data.remove(0);
        value |= ((current_byte & SEGMENT_BITS) as i64) << position;

        if (current_byte & CONTINUE_BIT) == 0 {
            break;
        }

        position += 7;

        if position >= size {
            return Err(Box::new(VarIntError::ValueTooLarge));
        }
    }

    Ok(value)
}
fn convert_var(mut value: i64) -> Vec<u8> {
    let mut data: Vec<u8> = vec![];
    loop {
        if (value & !(SEGMENT_BITS as i64)) == 0 {
            data.append(&mut vec![value as u8]);
            return data;
        }
        data.append(
            &mut vec![(value & (SEGMENT_BITS as i64)) as u8 | CONTINUE_BIT]);
        value >>= 7;
    }
}

pub fn get_string(data: &mut Vec<u8>) -> Result<String> {
    let length = get_var_int(data)? as usize;
    let buffer = data[..length].to_vec();
    for _ in 0..length { data.remove(0); }
    Ok(String::from_utf8_lossy(&buffer).to_string())
}
pub fn convert_string(s: &str) -> Vec<u8> {
    let length = s.len() as i32;
    let mut data = convert_var_int(length);
    data.append(&mut s.as_bytes().to_vec());
    data
}
