use std::fmt::Debug;

use thiserror::Error;

#[derive(Error)]
pub enum Error {
    #[error(transparent)]
    Bluetooth(#[from] bluer::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("invalid byte at {position:#x} (expected {expected}, got {actual})")]
    Parse { expected: &'static str, position: usize, actual: u8 },

    #[error("invalid data length (expected {expected}, got {actual})")]
    Length { expected: usize, actual: usize },

    // #[error("command not found - `{0}`")]
    // CommandNotFound(Command),

    #[error("")]
    Nothing
}

impl Error {
    pub fn vec_len<T>(v: Vec<u8>) -> Self {
        Error::Length { expected: std::mem::size_of::<T>(), actual: v.len() }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use bluer::ErrorKind::*;

        match self {
            Error::Bluetooth(e) => {
                match e.kind {
                    NotFound => write!(f, "couldn't find a bluetooth adapter."),
                    _ => e.fmt(f)
                }
            }
            _ => self.fmt(f)
        }
    }
}
