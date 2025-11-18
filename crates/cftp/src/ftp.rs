use crate::{
    FtpBuilder, FtpHandler, IoFactory, Security,
    code::{FtpResponse, IntoFtpResponse, SimpleReturnCode},
    command::Command,
};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::{collections::HashSet, error::Error, path::PathBuf};
use thiserror::Error;

#[cfg(feature = "tls")]
use crate::command::auth::{Auth, AuthType};
#[cfg(feature = "tls")]
use crate::tls::MaybeTls;
#[cfg(feature = "tls")]
use futures_rustls::TlsAcceptor;

#[derive(Debug, Error)]
pub enum HandleError<HandleErr>
where
    HandleErr: Error,
{
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("handler error: {0}")]
    Handler(HandleErr),
    #[error(transparent)]
    Read(#[from] ReadError),
    #[cfg(feature = "tls")]
    #[error("TLS upgrade error: {0}")]
    TlsUpgrade(#[from] TlsUpgradeError),
}

#[cfg(feature = "tls")]
#[derive(Debug, Error)]
pub enum TlsUpgradeError {
    #[error("TLS acceptor not configured")]
    Unconfigured,
    #[error("TLS upgrade previously failed")]
    PreviousFailure,
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Ftp<Handler, Stream>
where
    Handler: FtpHandler,
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    handler: Handler,
    #[cfg(feature = "tls")]
    reader: MaybeTls<Stream>,
    #[cfg(feature = "tls")]
    acceptor: Option<TlsAcceptor>,
    #[cfg(not(feature = "tls"))]
    reader: Stream,
    io_factory: Option<Handler::Factory>,
    #[cfg(feature = "tls")]
    allow_plaintext: bool,
}

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse command: {0}")]
    Parse(String),
}

impl<Handler, Stream> Ftp<Handler, Stream>
where
    Handler: FtpHandler<Io = Stream>,
    Stream: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn builder() -> FtpBuilder<Handler, Stream> {
        FtpBuilder::default()
    }

    pub async fn new_from_builder(
        handler: Handler,
        stream: Stream,
        builder: FtpBuilder<Handler, Stream>,
    ) -> std::io::Result<Self> {
        match builder.security {
            Security::NoEncryption => Ok(Self::new_insecure(handler, stream)),
            #[cfg(feature = "tls")]
            Security::Encryption(enc_info) => {
                let acceptor = TlsAcceptor::from(enc_info.config);
                match enc_info.implicit {
                    true => {
                        let tls_stream = acceptor.accept(stream).await?;
                        Ok(Ftp {
                            handler,
                            reader: MaybeTls::Tls(Box::new(tls_stream)),
                            io_factory: None,
                            acceptor: Some(acceptor),
                            allow_plaintext: enc_info.allow_plaintext,
                        })
                    }

                    false => Ok(Ftp {
                        handler,
                        reader: MaybeTls::Plain(stream),
                        io_factory: None,
                        acceptor: Some(acceptor),
                        allow_plaintext: enc_info.allow_plaintext,
                    }),
                }
            }
        }
    }

    pub fn new_insecure(handler: Handler, stream: Stream) -> Self {
        Ftp {
            handler,
            #[cfg(feature = "tls")]
            acceptor: None,
            #[cfg(feature = "tls")]
            reader: MaybeTls::Plain(stream),
            #[cfg(not(feature = "tls"))]
            reader: stream,
            io_factory: None,
            #[cfg(feature = "tls")]
            allow_plaintext: true,
        }
    }

    pub async fn handle(mut self) -> Result<(), HandleError<Handler::Err>> {
        let welcome = self.handler.welcome().await;
        self.write(FtpResponse::ReadyForNewUser(welcome)).await?;

        let user = loop {
            let command = match self.read().await? {
                CommandRead::Command(c) => c,
                CommandRead::Disconnect => {
                    tracing::info!("client disconnected before authentication");
                    return Ok(());
                }
            };

            match command {
                #[cfg(feature = "tls")]
                Command::Auth(Auth {
                    auth_type: AuthType::Tls,
                }) if self.acceptor.is_some() => {
                    tracing::info!("starting TLS handshake");
                    // upgrade to TLS, send 234
                    self.write(FtpResponse::simple_msg(
                        SimpleReturnCode::AuthenticationSuccessful,
                        "Starting TLS negotiation.",
                    ))
                    .await?;

                    self.upgrade_tls().await?;

                    continue;
                }

                #[cfg(feature = "tls")]
                Command::Auth(auth) => {
                    tracing::info!(?auth, "unsupported AUTH command received");
                    self.write(FtpResponse::simple_msg(
                        SimpleReturnCode::CommandNotImplemented,
                        "unsupported AUTH type",
                    ))
                    .await?;
                    continue;
                }

                #[cfg(not(feature = "tls"))]
                Command::Auth(_) => {
                    tracing::warn!("received AUTH command, but TLS support is not compiled in");
                    self.write(FtpResponse::simple_msg(
                        SimpleReturnCode::CommandNotImplemented,
                        "TLS not supported",
                    ))
                    .await?;
                    continue;
                }

                #[cfg(feature = "tls")]
                Command::User(_)
                    if !matches!(self.reader, MaybeTls::Tls(_)) && !self.allow_plaintext =>
                {
                    tracing::warn!(
                        "received USER command before TLS upgrade, but plaintext not allowed"
                    );
                    self.write(FtpResponse::simple_msg(
                        SimpleReturnCode::NotLoggedIn,
                        "Please use AUTH TLS before sending USER command.",
                    ))
                    .await?;
                }

                Command::User(user) => {
                    tracing::info!("received USER command for user: {}", user.username);
                    break user;
                }

                Command::Opts(ref opts) => {
                    tracing::info!(%opts.options, "received OPTS command before authentication");
                    self.write(FtpResponse::simple(SimpleReturnCode::Ok))
                        .await?;
                    continue;
                }

                command => {
                    tracing::error!(?command, "unsupported command received");
                    self.write(FtpResponse::simple(SimpleReturnCode::BadSequence))
                        .await?;
                    return Ok(());
                }
            };
        };

        self.write(FtpResponse::simple(SimpleReturnCode::NeedPassword))
            .await?;
        // wait for password command
        let CommandRead::Command(Command::Pass(pass)) = self.read().await? else {
            tracing::error!("expected PASS command");
            self.write(FtpResponse::simple(SimpleReturnCode::BadSequence))
                .await?;
            return Ok(());
        };

        if !self
            .handler
            .authenticate(&user.username, &pass.password)
            .await
            .map_err(HandleError::Handler)?
        {
            tracing::error!("authentication failed for user: {}", user.username);
            self.write(FtpResponse::simple(SimpleReturnCode::NotLoggedIn))
                .await?;
            return Ok(());
        }

        let mut to_rename = None;

        self.write(FtpResponse::simple(SimpleReturnCode::UserLoggedIn))
            .await?;

        loop {
            let command = match self.read().await {
                Ok(CommandRead::Command(command)) => command,
                Ok(CommandRead::Disconnect) => {
                    tracing::info!("client disconnected");
                    break;
                }
                Err(ReadError::Parse(parse_error)) => {
                    tracing::error!(err = %parse_error, "failed to parse command");
                    self.write(FtpResponse::simple_msg(
                        SimpleReturnCode::CommandNotImplemented,
                        parse_error,
                    ))
                    .await?;
                    continue;
                }
                Err(e) => {
                    tracing::error!(err = %e, "failed to read command");
                    break;
                }
            };

            tracing::info!(command = ?command, "handling command");

            match command {
                Command::Pwd(_) => {
                    let path = self.handler.cwd().await;
                    if let Some(path) = path {
                        let unix_path = path.to_string_lossy().replace('\\', "/");
                        tracing::info!("current working directory: {}", unix_path);
                        self.write(FtpResponse::DirectoryCreated(unix_path)).await?; // little bit weird
                    } else {
                        self.write(FtpResponse::simple(SimpleReturnCode::FileUnavailable))
                            .await?;
                    }
                }

                Command::Cwd(cwd) => {
                    if !self.handler.set_cwd(&PathBuf::from(cwd.path)).await {
                        self.write(FtpResponse::simple(SimpleReturnCode::FileUnavailable))
                            .await?;
                        continue;
                    }

                    self.write(FtpResponse::simple(SimpleReturnCode::Ok))
                        .await?;
                }

                Command::Type(t) => {
                    tracing::info!("supposed to change to {:?}", t.change_to);
                    self.write(FtpResponse::simple(SimpleReturnCode::Ok))
                        .await?;
                }

                Command::Pasv(_) => {
                    tracing::info!("establishing passive connection");

                    // let Some(passive_conn) = self.handler.passive_conn().await else {
                    //     tracing::error!("failed to establish passive connection");
                    //     self.write(Response::simple(SimpleReturnCode::CommandNotImplemented)).await?;
                    //     continue;
                    // };

                    let passive_conn = match self.handler.passive_conn().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            tracing::error!(err = %e, "failed to establish passive connection");
                            self.write(FtpResponse::simple(
                                SimpleReturnCode::CommandNotImplemented,
                            ))
                            .await?;
                            continue;
                        }
                    };

                    if let Some(reply) = passive_conn.to_reply() {
                        self.write(reply).await?;
                        self.io_factory = Some(passive_conn.into_inner());
                        tracing::info!("passive connection reply sent");
                    } else {
                        self.write(FtpResponse::simple(SimpleReturnCode::CommandNotImplemented))
                            .await?;
                        tracing::error!("failed to create passive connection reply");
                    }
                }

                Command::List(_) => {
                    let Some(mut data_stream) = self.passive_conn().await else {
                        tracing::error!("no passive connection available for LIST command");
                        self.write(FtpResponse::simple(SimpleReturnCode::CommandNotImplemented))
                            .await?;
                        continue;
                    };

                    let ls = match self.handler.ls().await {
                        Ok(ls) => ls,
                        Err(e) => {
                            tracing::error!(err = %e, "failed to get file listing");
                            self.write(FtpResponse::simple(
                                SimpleReturnCode::ClosingDataConnectionNoTransfer,
                            ))
                            .await?;
                            continue;
                        }
                    };

                    for file in ls {
                        if let Err(e) = data_stream
                            .write_all(format!("{file}\r\n").as_bytes())
                            .await
                        {
                            tracing::error!(err = %e, "failed to send LIST data");
                            self.write(FtpResponse::simple(
                                SimpleReturnCode::ClosingDataConnectionNoTransfer,
                            ))
                            .await?;
                            continue;
                        }
                    }

                    self.write(FtpResponse::simple(
                        SimpleReturnCode::ClosingDataConnectionSuccessful,
                    ))
                    .await?;
                }

                Command::Syst(_) => {
                    let os_info = self.handler.os_info().await;
                    self.write(FtpResponse::NameSystemType(os_info)).await?;
                }

                Command::Retr(retr) => {
                    let Some(mut data_stream) = self.passive_conn().await else {
                        tracing::error!("no passive connection available for RETR command");
                        self.write(FtpResponse::simple(SimpleReturnCode::CommandNotImplemented))
                            .await?;
                        continue;
                    };

                    match self.handler.read(&retr.file, &mut data_stream).await {
                        Ok(()) => {
                            tracing::info!(
                                name = %retr.file.display(),
                                "successfully sent file",
                            );
                            self.write(FtpResponse::simple(
                                SimpleReturnCode::ClosingDataConnectionSuccessful,
                            ))
                            .await?;
                        }
                        Err(e) => {
                            tracing::error!(err = %e, name = %retr.file.display(), "failed to read file");
                            self.write(FtpResponse::simple(SimpleReturnCode::FileUnavailable))
                                .await?;
                        }
                    }
                }

                Command::Stor(stor) => {
                    let Some(mut data_stream) = self.passive_conn().await else {
                        tracing::error!("no passive connection available for STOR command");
                        self.write(FtpResponse::simple(SimpleReturnCode::CommandNotImplemented))
                            .await?;
                        continue;
                    };

                    match self.handler.write(&stor.file, &mut data_stream).await {
                        Ok(()) => {
                            tracing::info!(
                                name = %stor.file.display(),
                                "successfully received file ",
                            );
                            self.write(FtpResponse::simple(
                                SimpleReturnCode::ClosingDataConnectionSuccessful,
                            ))
                            .await?;
                        }
                        Err(e) => {
                            tracing::error!(err = %e, name = %stor.file.display(), "failed to write file");
                            self.write(FtpResponse::simple(SimpleReturnCode::FileUnavailable))
                                .await?;
                        }
                    }
                }

                Command::Feat(_) => {
                    let mut features = self.handler.features().await;
                    // we also support UTF8, SIZE, MDTM, MFMT, MLST and MLSD
                    let default_features = vec!["UTF8", "SIZE", "MDTM", "MFMT", "MLST", "MLSD"]
                        .into_iter()
                        .map(String::from)
                        .collect::<HashSet<String>>();

                    features.extend(default_features);

                    self.write(FtpResponse::Features(features)).await?;
                }

                Command::Opts(opts) => {
                    tracing::info!(%opts.options, "received OPTS command");
                    // For simplicity, we just acknowledge the OPTS command without processing options
                    self.write(FtpResponse::simple(SimpleReturnCode::Ok))
                        .await?;
                }

                Command::Utf8(_) => {
                    self.write(FtpResponse::simple(SimpleReturnCode::Ok))
                        .await?;
                }

                Command::Auth(_) | Command::User(_) | Command::Pass(_) => {
                    tracing::warn!("received unexpected authentication command after login");
                    self.write(FtpResponse::simple(SimpleReturnCode::BadSequence))
                        .await?;
                }

                Command::Rnfr(rnfr) => {
                    tracing::info!(path = %rnfr.path.display(), "received RNFR command");
                    self.write(FtpResponse::simple(SimpleReturnCode::FileActionPending))
                        .await?;

                    to_rename = Some(rnfr.path);
                }

                Command::Rnto(r) => {
                    let Some(from_path) = to_rename.take() else {
                        tracing::error!("RNT0 command received without preceding RNFR");
                        self.write(FtpResponse::simple(SimpleReturnCode::BadSequence))
                            .await?;
                        continue;
                    };

                    match self.handler.rename(&from_path, &r.to).await {
                        Ok(()) => {
                            tracing::info!(
                                from = %from_path.display(),
                                to = %r.to.display(),
                                "successfully renamed file",
                            );
                            self.write(FtpResponse::simple(SimpleReturnCode::Ok))
                                .await?;
                        }
                        Err(e) => {
                            tracing::error!(
                                err = %e,
                                from = %from_path.display(),
                                to = %r.to.display(),
                                "failed to rename file",
                            );
                            self.write(e.into_ftp_response()).await?;
                        }
                    }
                }

                Command::Pbsz(_) => {
                    self.write(FtpResponse::simple(SimpleReturnCode::Ok))
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn passive_conn(&mut self) -> Option<Handler::Io> {
        self.write(FtpResponse::simple(SimpleReturnCode::OpeningDataConnection))
            .await
            .ok()?;
        self.io_factory.as_mut()?.create_io().await
    }

    #[cfg(feature = "tls")]
    async fn upgrade_tls(&mut self) -> Result<(), TlsUpgradeError> {
        // move out of self.reader is ok as long as we put something back
        match &mut self.reader {
            MaybeTls::Plain(_) => {
                // swap with an upgrading state
                let stream = std::mem::replace(&mut self.reader, MaybeTls::UpgradeBroken);
                if let MaybeTls::Plain(s) = stream
                    && let Some(acceptor) = &self.acceptor
                {
                    let tls_stream = acceptor.accept(s).await?;
                    self.reader = MaybeTls::Tls(Box::new(tls_stream));
                    Ok(())
                } else {
                    Err(TlsUpgradeError::Unconfigured)
                }
            }

            MaybeTls::Tls(_) => {
                tracing::warn!("TLS upgrade requested, but connection is already TLS");
                Ok(())
            }

            MaybeTls::UpgradeBroken => {
                tracing::error!("TLS upgrade previously failed");
                // Err("TLS upgrade previously failed".into())
                Err(TlsUpgradeError::PreviousFailure)
            }
        }
    }

    async fn read(&mut self) -> Result<CommandRead, ReadError> {
        let mut buf = Vec::new();

        loop {
            let mut byte = [0u8; 1];

            let n = self.reader.read(&mut byte).await?;
            if n == 0 {
                break; // EOF
            }

            buf.push(byte[0]);
            let len = buf.len();
            if len >= 2 && buf[len - 2..] == *b"\r\n" {
                buf.truncate(len - 2);
                break;
            }
        }

        let command_str = String::from_utf8(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        if command_str.trim().is_empty() {
            return Ok(CommandRead::Disconnect);
        }

        command_str
            .parse()
            .map_err(ReadError::Parse)
            .map(CommandRead::Command)
    }

    async fn write(&mut self, msg: FtpResponse) -> Result<(), std::io::Error> {
        self.write_bytes(&msg.to_bytes()).await
    }

    async fn write_bytes(&mut self, data: &[u8]) -> Result<(), std::io::Error> {
        self.reader.write_all(data).await?;
        self.reader.flush().await
    }
}

enum CommandRead {
    Command(Command),
    Disconnect,
}
