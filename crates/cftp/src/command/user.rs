use std::{convert::Infallible, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct User {
    pub username: String,
}

impl FromStr for User {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            username: s.trim().to_string(),
        })
    }
}
