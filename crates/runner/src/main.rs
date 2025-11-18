//! this crate shows a simple FTP server implementation using the cftp crate.

mod handler;

use crate::handler::Handler;
use cftp::{
    EncryptionInfo, Ftp,
    io::TokioAsyncReadCompatExt,
    pki_types::{
        PrivateKeyDer,
        pem::{PemObject, SectionKind},
    },
    rustls::ServerConfig,
};
use rcgen::CertifiedKey;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("runner=debug,cftp=debug")
        .init();

    let listener = TcpListener::bind("127.0.0.1:21").await?;
    tracing::info!("FTP server listening on 127.0.0.1:21");

    // generate temporary tls certs
    let CertifiedKey { cert, signing_key } = rcgen::generate_simple_self_signed(vec![])?;
    let key = PrivateKeyDer::from_pem(SectionKind::PrivateKey, signing_key.serialize_der())
        .ok_or_else(|| anyhow::anyhow!("failed to parse private key"))?;

    // boring rustls stuff
    let config = Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.der().clone()], key)?,
    );

    while let Ok((stream, addr)) = listener.accept().await {
        let ftp = Ftp::builder()
            .encryption(EncryptionInfo::builder(config.clone()).build())
            .build(Handler, stream.compat()) // .compat() to convert Tokio TcpStream to futures AsyncRead + AsyncWrite, because cftp is runtime-agnostic
            .await?;

        tokio::spawn(async move {
            if let Err(e) = ftp.handle().await {
                tracing::error!(%addr, %e, "error handling client");
            }
        });
    }

    Ok(())
}
