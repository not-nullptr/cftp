//! cftp allows you to build a fully customizable FTP server. it only handles the protocol-side
//! of things, so you need to provide your own implementation for file storage, authentication, etc.
//!
//! this file demonstrates an implementation of the `FtpHandler` trait that does nothing and always
//! returns empty results. you can use this as a starting point for your own implementation. it also
//! demonstrates `TcpFactory` (enabled via the "tcp" feature) for passive connections over TCP with
//! Tokio. does anyone else find it really hard to write comments without sounding like an llm LMAO

#![allow(unused)]

use cftp::{
    FtpHandler,
    code::SimpleReturnCode,
    io::{AsyncRead, AsyncWrite, Compat},
    tcp::{TcpFactory, TcpFactoryError},
};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
};
use thiserror::Error;
use tokio::net::TcpStream;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("error creating passive connection: {0}")]
    PassiveConn(#[from] TcpFactoryError),
}

/// cftp has an `IntoFtpResponse` trait. it allows you to use a custom type
/// as an error type in your handler, which is then sent to the client.
/// notably, it is implemented for all types which implement
/// `Display + Into<SimpleReturnCode>`, so you can just return
/// a simple enum or struct that implements those traits (although you can
/// also manually implement `IntoFtpResponse`, if you want).
///
/// for example, if you had an error which serialised to "file not found",
/// and which mapped to the 550 return code in the `Into<SimpleReturnCode>` impl,
/// the client would receive:
///
/// `550 file not found\r\n`
///
/// i highly recommend you use `thiserror` or similar to define your error types, as
/// you get Display for free.
impl From<HandlerError> for SimpleReturnCode {
    fn from(error: HandlerError) -> Self {
        match error {
            HandlerError::PassiveConn(_) => SimpleReturnCode::ServiceNotAvailable,
        }
    }
}

/// a no-op handler that does nothing and always returns empty results.
///
/// note that the handler does not have to be empty -- each method receives
/// &mut self, so you can do whatever you want.
pub struct Handler;

impl FtpHandler for Handler {
    /// `Err` is the error type returned by handler methods.
    type Err = HandlerError;

    /// `Factory` is the type used to create passive connections.
    type Factory = TcpFactory;

    /// `Io` is the type of the I/O stream used for connections.
    type Io = Compat<TcpStream>;

    async fn authenticate(&mut self, username: &str, password: &str) -> Result<bool, Self::Err> {
        Ok(true)
    }

    /// implementations are generally expected to store the cwd internally.
    async fn cwd(&mut self) -> Option<&Path> {
        Some(Path::new("/"))
    }

    async fn ls(&mut self) -> Result<Vec<cftp::FileListing>, Self::Err> {
        Ok(vec![cftp::FileListing {
            name: "file1.txt".to_string(),
            is_dir: false,
            size: 1234,
            modified: chrono::Utc::now(),
            permissions: 0o644,
            owner: "user".to_string(),
            group: "group".to_string(),
        }])
    }

    /// this method is used to construct a passive connection for data transfer.
    ///
    /// cftp currently does not support active connections due to security concerns.
    async fn passive_conn(
        &mut self,
    ) -> Result<cftp::PassiveConn<Self::Io, Self::Factory>, Self::Err> {
        Ok(TcpFactory::bind(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))).await?)
    }

    async fn rename(&mut self, from: &Path, to: &Path) -> Result<(), Self::Err> {
        Ok(())
    }

    async fn set_cwd(&mut self, path: &Path) -> bool {
        false
    }

    /// cftp allows you to stream reads and writes. this could be a simple `tokio::fs::File`,
    /// or something more complex like an in-memory buffer, or a networked file system. all of it
    /// can be efficiently streamed without loading the entire file into memory using `tokio::io::copy`
    /// (or your runtime's equivalent).
    ///
    /// of course, you don't have to stream if your storage backend has no support, you're always
    /// allowed to just read it into a buffer. but you shouldn't ^_^
    async fn read<W>(&mut self, path: &Path, writer: &mut W) -> Result<(), Self::Err>
    where
        W: AsyncWrite + Unpin + Send,
    {
        Ok(())
    }

    /// similarly to `read`, this method allows you to stream data from the client to your storage backend.
    /// see the comments above for more details.
    async fn write<R>(&mut self, path: &Path, reader: &mut R) -> Result<(), Self::Err>
    where
        R: AsyncRead + Unpin + Send,
    {
        Ok(())
    }
}
