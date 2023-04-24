use std::error;
use std::fmt;
use std::io;

use crate::backend;

#[derive(Debug)]
pub enum FrontendError {
    Play(rodio::PlayError),
    Backend(backend::BackendError),
    IO(io::Error),
}

impl FrontendError {
    pub fn is_fatal(&self) -> bool {
        match self {
            Self::Backend(error) => matches!(
                error.kind,
                backend::BackendErrorKind::MemoryOverflow
                    | backend::BackendErrorKind::ProgramInvalid
                    | backend::BackendErrorKind::ProgramNotLoaded
            ),
            _ => true,
        }
    }
}

impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Play(error) => write!(f, "{}", error),
            Self::Backend(error) => write!(f, "{}", error),
            Self::IO(error) => write!(f, "{}", error),
        }
    }
}

impl error::Error for FrontendError {}
