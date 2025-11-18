use std::{convert::Infallible, path::PathBuf, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rnto {
    pub to: PathBuf,
}

impl FromStr for Rnto {
    type Err = Infallible;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            to: PathBuf::from(path.replace("\\", "/")),
        })
    }
}
