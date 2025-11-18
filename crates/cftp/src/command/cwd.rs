use std::{convert::Infallible, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Cwd {
    pub path: String,
}

impl FromStr for Cwd {
    type Err = Infallible;

    fn from_str(c: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            path: c.trim().to_string(),
        })
    }
}
