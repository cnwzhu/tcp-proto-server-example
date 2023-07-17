use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("禁止连接")]
    ConnectionProhibited,

    #[error("IO错误")]
    IoError(#[from] std::io::Error),

    #[error("连接不存在")]
    ConnectionNotFound,

    #[error("连接已关闭")]
    ConnectionClosed,

    #[error("读取数据为空")]
    ReadDataEmpty,

    #[error("未知消息类型")]
    UnknownMessageType,
}