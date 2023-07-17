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
    fn read(stream: &BytesMut) -> error::Result<Header> {
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
pub struct HeartBeatRequestMessage {}

impl RequestMessage {
    pub fn read(stream: &BytesMut) -> error::Result<Option<RequestMessage>> {
        let header = Header::read(stream)?;
        match header.msg_type {
            0x01 => Ok(Some(RequestMessage::Auth(AuthRequestMessage::read(
                header,
            )?))),
            _ => {
                tracing::error!("unknown message type: {}", header.msg_type);
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_auth_request_message() {
        let raw_data = vec![
            0x32, 0xB1, 0xDF, 0x00, 0x07, 0x02, 0x01, 0x31, 0x01, 0x31, 0x01, 0x31, 0x35, 0x66,
        ];
        let header = Header {
            raw_data: BytesMut::from_iter(raw_data.iter()),
            msg_type: 0x01,
            msg_seq: 11,
            msg_len: 12,
            msg_crc: 0x6c6b,
        };
        let auth_request_message = AuthRequestMessage::read(header).unwrap();
        assert_eq!(auth_request_message.auth_type, 1);
        assert_eq!(auth_request_message.connect_code, "abcdefghijkl");
        assert_eq!(auth_request_message.connect_id, "");
    }

    #[test]
    fn test_read_unknown_message_type() {
        let raw_data = vec![0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let header = Header {
            raw_data: BytesMut::from_iter(raw_data.iter()),
            msg_type: 0x02,
            msg_seq: 0,
            msg_len: 0,
            msg_crc: 0,
        };
        let request_message = RequestMessage::read(&header.raw_data).unwrap();
        assert!(request_message.is_none());
    }

    #[test]
    fn test_read_auth_request_message_with_invalid_data() {
        let raw_data = vec![0x01, 0x00, 0x01, 0x00, 0x00, 0x00];
        let header = Header {
            raw_data: BytesMut::from_iter(raw_data.iter()),
            msg_type: 0x01,
            msg_seq: 1,
            msg_len: 0,
            msg_crc: 0,
        };
        let auth_request_message = AuthRequestMessage::read(header);
        assert!(auth_request_message.is_err());
    }
}
