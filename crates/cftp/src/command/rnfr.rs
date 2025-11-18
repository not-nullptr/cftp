use std::{convert::Infallible, path::PathBuf, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rnfr {
    pub path: PathBuf,
}

impl FromStr for Rnfr {
    type Err = Infallible;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            path: PathBuf::from(path.replace("\\", "/")),
        })
    }
}
