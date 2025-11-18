use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Auth {
    pub auth_type: AuthType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthType {
    Ssl,
    Tls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum AuthTypeParseError {
    #[error("invalid authentication type")]
    InvalidAuthType,
}

impl FromStr for AuthType {
    type Err = AuthTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("SSL") {
            Ok(AuthType::Ssl)
        } else if s.eq_ignore_ascii_case("TLS") {
            Ok(AuthType::Tls)
        } else {
            Err(AuthTypeParseError::InvalidAuthType)
        }
    }
}

impl FromStr for Auth {
    type Err = AuthTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let auth_type = s.trim().parse()?;
        Ok(Self { auth_type })
    }
}
