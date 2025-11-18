use std::{convert::Infallible, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pass {
    pub password: String,
}

impl FromStr for Pass {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            password: s.trim().to_string(),
        })
    }
}
