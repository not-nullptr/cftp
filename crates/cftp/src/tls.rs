use futures::{AsyncRead, AsyncWrite};
use futures_rustls::server::TlsStream;

pub enum MaybeTls<Stream>
where
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    Plain(Stream),
    Tls(Box<TlsStream<Stream>>),
    UpgradeBroken,
}

impl<Stream> AsyncRead for MaybeTls<Stream>
where
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            MaybeTls::Tls(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            MaybeTls::UpgradeBroken => {
                std::task::Poll::Ready(Err(std::io::Error::other("TLS upgrade failed")))
            }
        }
    }
}

impl<Stream> AsyncWrite for MaybeTls<Stream>
where
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            MaybeTls::Tls(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            MaybeTls::UpgradeBroken => {
                std::task::Poll::Ready(Err(std::io::Error::other("TLS upgrade failed")))
            }
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            MaybeTls::Tls(s) => std::pin::Pin::new(s).poll_flush(cx),
            MaybeTls::UpgradeBroken => {
                std::task::Poll::Ready(Err(std::io::Error::other("TLS upgrade failed")))
            }
        }
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            MaybeTls::Plain(s) => std::pin::Pin::new(s).poll_close(cx),
            MaybeTls::Tls(s) => std::pin::Pin::new(s).poll_close(cx),
            MaybeTls::UpgradeBroken => {
                std::task::Poll::Ready(Err(std::io::Error::other("TLS upgrade failed")))
            }
        }
    }
}
