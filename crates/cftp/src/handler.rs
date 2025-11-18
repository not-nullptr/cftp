use std::{collections::HashSet, error::Error, fmt, net::SocketAddr, path::Path};

use chrono::{DateTime, Datelike, Timelike, Utc};
use futures::{AsyncRead, AsyncWrite};

use crate::code::{FtpResponse, IntoFtpResponse, Port};

pub trait FtpHandler: Send + Sync {
    type Io: AsyncRead + AsyncWrite + Unpin;
    type Factory: IoFactory<Io = Self::Io>;
    type Err: Error + IntoFtpResponse + Send + Sync + 'static;

    fn welcome(&mut self) -> impl Future<Output = String> {
        async { "cftp by nullptr".to_string() }
    }

    fn authenticate(
        &mut self,
        username: &str,
        password: &str,
    ) -> impl Future<Output = Result<bool, Self::Err>>;

    fn cwd(&mut self) -> impl Future<Output = Option<&Path>>;
    fn set_cwd(&mut self, path: &Path) -> impl Future<Output = bool>;
    fn ls(&mut self) -> impl Future<Output = Result<Vec<FileListing>, Self::Err>>;
    fn rename(&mut self, from: &Path, to: &Path) -> impl Future<Output = Result<(), Self::Err>>;

    fn passive_conn(
        &mut self,
    ) -> impl Future<Output = Result<crate::PassiveConn<Self::Io, Self::Factory>, Self::Err>>;

    fn os_info(&mut self) -> impl Future<Output = String> {
        async { "UNIX Type: L8".to_string() }
    }

    fn features(&mut self) -> impl Future<Output = HashSet<String>> {
        async { HashSet::new() }
    }

    fn read<W>(
        &mut self,
        path: &Path,
        writer: &mut W,
    ) -> impl Future<Output = Result<(), Self::Err>>
    where
        W: AsyncWrite + Unpin + Send;

    fn write<R>(
        &mut self,
        path: &Path,
        reader: &mut R,
    ) -> impl Future<Output = Result<(), Self::Err>>
    where
        R: AsyncRead + Unpin + Send;
}

// this trait could be better-designed (type Err, Option<T> -> Result<T, Err>) but i cba right now
pub trait IoFactory {
    type Io: AsyncRead + AsyncWrite + Unpin;
    fn create_io(&mut self) -> impl Future<Output = Option<Self::Io>>;
}

pub struct PassiveConn<Io, Factory>
where
    Io: AsyncRead + AsyncWrite,
    Factory: IoFactory<Io = Io>,
{
    io_factory: Factory,
    addr: SocketAddr,
}

impl<Io, Factory> PassiveConn<Io, Factory>
where
    Io: AsyncRead + AsyncWrite,
    Factory: IoFactory<Io = Io>,
{
    pub fn new(addr: SocketAddr, io_factory: Factory) -> Self {
        Self { addr, io_factory }
    }

    pub async fn create_io(&mut self) -> Option<Io> {
        self.io_factory.create_io().await
    }

    pub fn into_inner(self) -> Factory {
        self.io_factory
    }

    pub fn to_reply(&self) -> Option<FtpResponse> {
        let ipv4 = match self.addr.ip() {
            std::net::IpAddr::V4(ipv4) => ipv4,
            std::net::IpAddr::V6(_) => {
                tracing::error!("Passive connection address is not IPv4");
                return None;
            }
        };

        let port = self.addr.port();
        Some(FtpResponse::EnteringPassiveMode(ipv4, Port(port)))
    }
}

#[derive(Debug, Clone)]
pub struct FileListing {
    pub name: String,
    pub is_dir: bool,
    pub permissions: u16,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub owner: String,
    pub group: String,
}

// this Display impl is best-effort; different FTP clients expect different formats and i can't do anything about that
impl fmt::Display for FileListing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ftype = if self.is_dir { 'd' } else { '-' };
        let perms = format!(
            "{}{}{}{}{}{}{}{}{}{}",
            ftype,
            if self.permissions & 0o400 != 0 {
                'r'
            } else {
                '-'
            },
            if self.permissions & 0o200 != 0 {
                'w'
            } else {
                '-'
            },
            if self.permissions & 0o100 != 0 {
                'x'
            } else {
                '-'
            },
            if self.permissions & 0o040 != 0 {
                'r'
            } else {
                '-'
            },
            if self.permissions & 0o020 != 0 {
                'w'
            } else {
                '-'
            },
            if self.permissions & 0o010 != 0 {
                'x'
            } else {
                '-'
            },
            if self.permissions & 0o004 != 0 {
                'r'
            } else {
                '-'
            },
            if self.permissions & 0o002 != 0 {
                'w'
            } else {
                '-'
            },
            if self.permissions & 0o001 != 0 {
                'x'
            } else {
                '-'
            },
        );

        let link_count = 1;

        let now = Utc::now();
        let six_months_secs = 6 * 30 * 24 * 60 * 60;
        let now_ts = now.timestamp();
        let mod_ts = self.modified.timestamp();

        let time_or_year = if (now_ts - mod_ts).abs() < six_months_secs {
            format!("{:02}:{:02}", self.modified.hour(), self.modified.minute())
        } else {
            format!("{:4}", self.modified.year())
        };

        let month = self.modified.format("%b").to_string();
        let day = format!("{:2}", self.modified.day());

        write!(
            f,
            "{perms} {links:>2} {owner} {group} {size:>8} {month} {day} {time_or_year} {name}",
            links = link_count,
            owner = self.owner,
            group = self.group,
            size = self.size,
            month = month,
            day = day,
            time_or_year = time_or_year,
            name = self.name,
        )
    }
}
