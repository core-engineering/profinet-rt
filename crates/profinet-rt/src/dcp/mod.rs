//! PROFINET DCP (Discovery & Configuration Protocol) — device side.

pub mod block;
pub mod frame;
pub mod identify;

use thiserror::Error;

/// Errors from parsing/serializing DCP frames.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DcpError {
    #[error("buffer too short: need {need}, have {have}")]
    TooShort { need: usize, have: usize },
    #[error("unknown DCP service id {0}")]
    BadServiceId(u8),
    #[error("unknown DCP service type {0}")]
    BadServiceType(u8),
    #[error("unknown DCP frame id {0:#06x}")]
    BadFrameId(u16),
    #[error("malformed DCP frame: {0}")]
    Malformed(&'static str),
}
