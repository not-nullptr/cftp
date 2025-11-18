use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Type {
    pub change_to: TransferType,
}

impl FromStr for Type {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let change_to = s.trim().parse()?;
        Ok(Self { change_to })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferType {
    Ascii,
    Binary,
}

impl FromStr for TransferType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "A" | "ASCII" => Ok(TransferType::Ascii),
            "I" | "BINARY" => Ok(TransferType::Binary),
            _ => Err("invalid transfer type"),
        }
    }
}
