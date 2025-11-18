use std::{convert::Infallible, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Opts {
    pub options: String,
}

impl FromStr for Opts {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            options: s.trim().to_string(),
        })
    }
}
