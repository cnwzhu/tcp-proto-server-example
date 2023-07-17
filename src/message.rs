use std::io::Cursor;

use byteorder::{BigEndian, ReadBytesExt};
use bytes::BytesMut;

use crate::error;

#[derive(Debug)]
pub enum RequestMessage {
    // 鉴权
    Auth(AuthRequestMessage),
    // 心跳
    HeartBeat(HeartBeatRequestMessage),
}

#[derive(Debug)]
pub struct Header {
    // 原始数据
    raw_data: BytesMut,
    // 消息类型
    msg_type: u8,
    // 消息序号
    msg_seq: u16,
    // 消息长度
    msg_len: u16,
    // 消息校验
    msg_crc: u16,
}

impl Header {
    async fn read(stream: &BytesMut) -> error::Result<Header> {
        let mut c = Cursor::new(stream);
        let msg_type = c.read_u8()?;
        let seq = c.read_u16::<BigEndian>()?;
        let len = c.read_u16::<BigEndian>()?;
        c.set_position(5 + len as u64);
        let crc = c.read_u16::<BigEndian>()?;
        Ok(Header {
            raw_data: stream.clone(),
            msg_type,
            msg_seq: seq,
            msg_len: len,
            msg_crc: crc,
        })
    }
}

#[derive(Debug)]
pub struct AuthRequestMessage {
    pub auth_type: u8,
    pub connect_code: String,
    pub connect_id: String,
}

impl AuthRequestMessage {
    pub fn read(header: Header) -> error::Result<Self> {
        let data = &header.raw_data;
        let mut cursor = Cursor::new(data);
        cursor.set_position(5);
        let auth_type = cursor.read_u8()?;
        let connect_code = read_str(&mut cursor)?;
        let connect_id = read_str(&mut cursor)?;
        Ok(AuthRequestMessage {
            auth_type,
            connect_code,
            connect_id,
        })
    }
}

fn read_str(c: &mut Cursor<&BytesMut>) -> error::Result<String> {
    let len = c.read_u8()?;
    let mut s: Vec<char> = Vec::with_capacity(len as usize);
    for _ in 0..len {
        s.push(c.read_u8()? as char);
    }
    Ok(String::from_iter(s.iter()))
}

#[derive(Debug)]
pub struct HeartBeatRequestMessage {

}

impl RequestMessage {
    pub async fn read(stream: &BytesMut) -> error::Result<Option<RequestMessage>> {
        let header = Header::read(stream).await?;
        match header.msg_type {
            0x01 => {
                Ok(Some(RequestMessage::Auth(AuthRequestMessage::read(header)?)))
            }
            _ => {
                tracing::error!("unknown message type: {}", header.msg_type);
                Ok(None)
            }
        }
    }
}

