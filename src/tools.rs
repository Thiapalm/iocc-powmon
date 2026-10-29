use std::fmt::Display;

///Valid error codes
#[derive(Debug, PartialEq)]
pub enum Error {
    CommunicationErr,
    InvalidDie,
    InvalidManufacturer,
    InvalidValue(String),
}

impl From<mcp2221::Error> for Error {
    fn from(_: mcp2221::Error) -> Self {
        Error::CommunicationErr
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::InvalidDie => write!(f, "Invalid Die Number"),
            Error::CommunicationErr => write!(f, "Not found on address"),
            Error::InvalidManufacturer => write!(f, "Invalid Manufacturer"),
            Error::InvalidValue(msg) => write!(f, "{msg}"),
        }
    }
}
