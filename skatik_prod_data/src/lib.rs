#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate thiserror;

use std::sync::mpsc::{RecvError, SendError};

#[derive(Error, Debug)]
pub enum SkatikError {
    #[error("Error: {0}")]
    Custom(String),
    #[error("Pipe read error")]
    PipeRead,
    #[error("Pipe write error")]
    PipeWrite,
}

impl From<RecvError> for SkatikError {
    fn from(_: RecvError) -> Self {
        Self::PipeRead
    }
}

impl<T> From<SendError<T>> for SkatikError {
    fn from(_: SendError<T>) -> Self {
        Self::PipeWrite
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, new)]
pub struct AnchorId<const TABLE_ID: usize>(usize);
