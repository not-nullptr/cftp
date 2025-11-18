use std::{convert::Infallible, path::PathBuf, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Retr {
    pub file: PathBuf,
}

impl FromStr for Retr {
    type Err = Infallible;

    fn from_str(file: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            file: PathBuf::from(file.replace("\\", "/")),
        })
    }
}
