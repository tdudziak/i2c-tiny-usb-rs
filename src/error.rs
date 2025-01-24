#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(rusb::Error);

pub type Result<T> = std::result::Result<T, Error>;

impl From<rusb::Error> for Error {
    fn from(e: rusb::Error) -> Self {
        Self(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        Error(match err.kind() {
            ErrorKind::PermissionDenied => rusb::Error::Access,
            ErrorKind::ResourceBusy => rusb::Error::Busy,
            ErrorKind::TimedOut => rusb::Error::Timeout,
            _ => rusb::Error::Other,
        })
    }
}

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        use rusb::Error as RE;
        use std::io::ErrorKind;
        match value.0 {
            RE::Access => ErrorKind::PermissionDenied,
            RE::Busy => ErrorKind::ResourceBusy,
            RE::Timeout => ErrorKind::TimedOut,
            // TODO: handle more cases where it makes sense?
            _ => ErrorKind::Other,
        }
        .into()
    }
}
