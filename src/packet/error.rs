use std::io;
use std::io::{Error, ErrorKind};

#[derive(Debug)]
pub enum PacketError {
    Io(Error),
    Other(String),
}

impl From<Error> for PacketError {
    fn from(error: Error) -> Self {
        PacketError::Io(error)
    }
}

impl From<Error> for PacketError {
    fn from(err: Error) -> Self {
        Self::Io(err)
    }
}

pub(crate) fn error<T>(reason: String) -> Result<T, PacketError> {
    Err(PacketError::Io(Error::new(ErrorKind::Other, reason)))
}