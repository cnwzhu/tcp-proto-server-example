use std::io;
use std::io::{Cursor, Seek};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::{Buf, BufMut, BytesMut};
use crc::{Crc, CRC_16_MODBUS};

use tokio_util::codec::Decoder;

pub struct LengthDelimitedCodec;

impl Decoder for LengthDelimitedCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<BytesMut>> {
        let current_len = src.len();
        if current_len == 0 {
            return Ok(None);
        }

        if current_len < 5 {
            return Ok(None);
        }

        let mut c = Cursor::new(&src);
        c.seek(io::SeekFrom::Start(3))?;
        let len = c.read_u16::<BigEndian>()?;
        let all_len = (5 + len + 2) as usize;
        if current_len < all_len {
            return Ok(None);
        }

        c.seek(io::SeekFrom::Start(0))?;

        let mut dst = BytesMut::with_capacity(all_len);
        dst.put_slice(&c.get_ref()[0..all_len]);

        c.seek(io::SeekFrom::Start(5 + len as u64))?;
        let crc_data = c.read_u16::<LittleEndian>()?;

        static CRC_16: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);
        let calculate = CRC_16.checksum(&dst[0..dst.len() - 2]);
        tracing::debug!("calculate: {:x?}, crc_data: {:x?}", calculate, crc_data);

        if calculate != crc_data {
            tracing::error!("crc check failed");
            src.advance(current_len);
            return Ok(None);
        }
        src.advance(all_len);
        Ok(Some(dst))
    }
}