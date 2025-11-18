use std::net::IpAddr;

use crate::{IoFactory, PassiveConn};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt};

#[derive(Debug, Error)]
pub enum TcpFactoryError {
    #[error("failed to pick unused port for passive connection")]
    NoUnusedPort,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct TcpFactory {
    listener: TcpListener,
}

impl TcpFactory {
    pub fn new(listener: TcpListener) -> Self {
        Self { listener }
    }

    pub async fn bind(
        addr: IpAddr,
    ) -> Result<PassiveConn<Compat<tokio::net::TcpStream>, TcpFactory>, TcpFactoryError> {
        let port = match openport::pick_random_unused_port() {
            Some(p) => p,
            None => {
                tracing::error!("failed to pick unused port for passive connection");
                return Err(TcpFactoryError::NoUnusedPort);
            }
        };

        let listener = match TcpListener::bind((addr, port)).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(port, err = %e, "failed to bind passive connection listener");
                return Err(TcpFactoryError::Io(e));
            }
        };

        let addr = match listener.local_addr() {
            Ok(a) => a,
            Err(e) => {
                tracing::error!(err = %e, "failed to get local address of passive connection listener");
                return Err(TcpFactoryError::Io(e));
            }
        };

        tracing::info!("listening for passive connection on {}", addr);
        Ok(PassiveConn::new(addr, TcpFactory::new(listener)))
    }
}

impl IoFactory for TcpFactory {
    type Io = Compat<tokio::net::TcpStream>;

    async fn create_io(&mut self) -> Option<Self::Io> {
        let (stream, _) = match self.listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(err = %e, "failed to accept passive connection");
                return None;
            }
        };

        Some(stream.compat())
    }
}
