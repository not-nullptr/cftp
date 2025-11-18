use crate::{Ftp, FtpHandler};
use futures::{AsyncRead, AsyncWrite};
use std::marker::PhantomData;

#[cfg(feature = "tls")]
use futures_rustls::rustls::ServerConfig;

#[cfg(feature = "tls")]
use std::sync::Arc;

#[cfg(feature = "tls")]
pub struct EncryptionInfo {
    pub implicit: bool,
    pub allow_plaintext: bool,
    pub config: Arc<ServerConfig>,
}

#[cfg(feature = "tls")]
impl EncryptionInfo {
    pub fn builder(config: Arc<ServerConfig>) -> EncryptionBuilder {
        EncryptionBuilder::new(config)
    }
}

pub enum Security {
    NoEncryption,
    #[cfg(feature = "tls")]
    Encryption(EncryptionInfo),
}

pub struct FtpBuilder<Handler, Stream>
where
    Handler: FtpHandler,
    Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub(crate) security: Security,
    __phantom: PhantomData<(Handler, Stream)>,
}

impl<Handler, Stream> Default for FtpBuilder<Handler, Stream>
where
    Handler: FtpHandler<Io = Stream>,
    Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    fn default() -> Self {
        Self {
            __phantom: PhantomData,
            security: Security::NoEncryption,
        }
    }
}

impl<Handler, Stream> FtpBuilder<Handler, Stream>
where
    Handler: FtpHandler<Io = Stream>,
    Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    pub async fn build(
        self,
        handler: Handler,
        stream: Stream,
    ) -> std::io::Result<crate::Ftp<Handler, Stream>> {
        Ftp::new_from_builder(handler, stream, self).await
    }

    #[cfg(feature = "tls")]
    pub fn encryption(mut self, info: EncryptionInfo) -> Self {
        self.security = Security::Encryption(info);
        self
    }
}

#[cfg(feature = "tls")]
pub struct EncryptionBuilder {
    pub(crate) implicit: bool,
    pub(crate) allow_plaintext: bool,
    pub(crate) config: Arc<ServerConfig>,
}

#[cfg(feature = "tls")]
impl EncryptionBuilder {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        Self {
            implicit: false,
            allow_plaintext: false,
            config,
        }
    }

    pub fn implicit(mut self, implicit: bool) -> Self {
        self.implicit = implicit;
        self
    }

    pub fn allow_plaintext(mut self, allow: bool) -> Self {
        self.allow_plaintext = allow;
        self
    }

    pub fn build(self) -> EncryptionInfo {
        EncryptionInfo {
            implicit: self.implicit,
            allow_plaintext: self.allow_plaintext,
            config: self.config,
        }
    }
}
