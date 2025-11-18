use std::{collections::HashSet, fmt::Display, net::Ipv4Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Port(pub u16);

impl Port {
    pub fn p1_p2(self) -> (u8, u8) {
        let p1 = (self.0 >> 8) as u8;
        let p2 = (self.0 & 0xFF) as u8;
        (p1, p2)
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum_macros::EnumDiscriminants)]
pub enum SimpleReturnCode {
    RestartMarker = 110,
    ServiceReady = 120,
    DataConnectionAlreadyOpen = 125,
    OpeningDataConnection = 150,
    Ok = 200,
    Superfluous = 202,
    SystemStatus = 211,
    DirectoryStatus = 212,
    FileStatus = 213,
    HelpMessage = 214,
    ClosingControlConnection = 221,
    ClosingDataConnectionNoTransfer = 225,
    ClosingDataConnectionSuccessful = 226,
    UserLoggedIn = 230,
    AuthenticationSuccessful = 234,
    NeedPassword = 331,
    NeedAccount = 332,
    FileActionPending = 350,
    ServiceNotAvailable = 421,
    CantOpenDataConnection = 425,
    TransferAborted = 426,
    FileActionNotTaken = 450,
    LocalError = 451,
    InsufficientStorage = 452,
    SyntaxError = 501,
    CommandNotImplemented = 502,
    BadSequence = 503,
    ParameterNotImplemented = 504,
    NotLoggedIn = 530,
    NeetAccountForStoringFiles = 532,
    FileUnavailable = 550,
    ExceededStorageAllocation = 552,
    FilenameNotAllowed = 553,
}

#[repr(u16)]
#[derive(Debug, Clone, PartialEq, Eq, strum_macros::EnumDiscriminants)]
pub enum FtpResponse {
    Simple(SimpleReturnCode, Option<String>) = 0,
    Features(HashSet<String>) = 211,
    NameSystemType(String) = 215,
    ReadyForNewUser(String) = 220,
    EnteringPassiveMode(Ipv4Addr, Port) = 227,
    FileActionOk(Option<String>) = 250,
    DirectoryCreated(String) = 257,
}

impl FtpResponse {
    pub fn simple(code: SimpleReturnCode) -> Self {
        FtpResponse::Simple(code, None)
    }

    pub fn simple_msg(code: SimpleReturnCode, msg: impl Into<String>) -> Self {
        FtpResponse::Simple(code, Some(msg.into()))
    }

    pub fn code(&self) -> u16 {
        match self {
            FtpResponse::Simple(code, _) => *code as u16,
            _ => FtpResponseDiscriminants::from(self) as u16,
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        use std::io::Write;

        let mut buf = Vec::new();
        let code = self.code();

        let _ = write!(&mut buf, "{} ", code);

        match self {
            FtpResponse::ReadyForNewUser(msg)
            | FtpResponse::NameSystemType(msg)
            | FtpResponse::Simple(_, Some(msg)) => {
                let _ = write!(&mut buf, "{}", msg.replace("\"", r#"\""#));
            }

            FtpResponse::EnteringPassiveMode(ip, port) => {
                let octets = ip.octets();
                let (p1, p2) = port.p1_p2();
                let _ = write!(
                    &mut buf,
                    "({},{},{},{},{},{})",
                    octets[0], octets[1], octets[2], octets[3], p1, p2
                );
            }

            FtpResponse::FileActionOk(Some(path)) | FtpResponse::DirectoryCreated(path) => {
                let path = path.replace('"', r#"\""#);
                let _ = write!(&mut buf, "\"{}\"", path);
            }

            FtpResponse::Features(f) => {
                let _ = write!(&mut buf, "Features:\r\n");
                for feature in f {
                    let _ = write!(&mut buf, " {feature}\r\n");
                }
                let _ = write!(&mut buf, "{code} End");
            }

            FtpResponse::FileActionOk(None) | FtpResponse::Simple(_, None) => {}
        }

        let _ = write!(&mut buf, "\r\n");

        let _ = std::io::Write::flush(&mut buf);

        buf
    }
}

pub trait IntoFtpResponse {
    fn into_ftp_response(self) -> FtpResponse
    where
        Self: Sized,
    {
        FtpResponse::simple(SimpleReturnCode::LocalError)
    }
}

impl<T> IntoFtpResponse for T
where
    T: Display + Into<SimpleReturnCode>,
{
    fn into_ftp_response(self) -> FtpResponse
    where
        Self: Sized,
    {
        let msg = self.to_string();
        FtpResponse::simple_msg(self.into(), msg)
    }
}
