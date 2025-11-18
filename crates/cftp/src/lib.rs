#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "../../../README.md"))]

pub mod code;
pub mod command;

#[cfg(feature = "tcp")]
pub mod tcp;

mod builder;
mod ftp;
mod handler;

#[cfg(feature = "tls")]
mod tls;

pub use builder::*;
pub use ftp::*;
pub use handler::*;

pub mod io {
    pub use futures::io::{AsyncRead, AsyncWrite};

    #[cfg(feature = "tcp")]
    pub use tokio_util::compat::{
        Compat, FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt, TokioAsyncReadCompatExt,
        TokioAsyncWriteCompatExt,
    };
}

#[cfg(feature = "tls")]
pub use futures_rustls::*;
