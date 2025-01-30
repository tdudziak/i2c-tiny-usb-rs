#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("USB error")]
    Usb(#[from] rusb::Error),

    #[error("no acknowledgement from the i2c device")]
    Nack,
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        use std::io::ErrorKind;
        match value {
            Error::Usb(rusb::Error::InvalidParam) => ErrorKind::InvalidInput.into(),
            Error::Usb(rusb::Error::Access) => ErrorKind::PermissionDenied.into(),
            Error::Usb(rusb::Error::NoDevice) => ErrorKind::ConnectionRefused.into(),
            Error::Usb(rusb::Error::NotFound) => ErrorKind::Unsupported.into(),
            Error::Usb(rusb::Error::Busy) => ErrorKind::ResourceBusy.into(),
            Error::Usb(rusb::Error::Timeout) => ErrorKind::TimedOut.into(),
            Error::Usb(rusb::Error::Pipe) => ErrorKind::BrokenPipe.into(),
            Error::Usb(rusb::Error::Interrupted) => ErrorKind::Interrupted.into(),
            Error::Usb(rusb::Error::NoMem) => ErrorKind::OutOfMemory.into(),
            Error::Usb(rusb::Error::NotSupported) => ErrorKind::InvalidInput.into(),
            Error::Usb(_) => ErrorKind::Other.into(),
            Error::Nack => ErrorKind::NotConnected.into(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::InvalidInput => Error::Usb(rusb::Error::InvalidParam),
            std::io::ErrorKind::PermissionDenied => Error::Usb(rusb::Error::Access),
            std::io::ErrorKind::ConnectionRefused => Error::Usb(rusb::Error::NoDevice),
            std::io::ErrorKind::Unsupported => Error::Usb(rusb::Error::NotFound),
            std::io::ErrorKind::ResourceBusy => Error::Usb(rusb::Error::Busy),
            std::io::ErrorKind::TimedOut => Error::Usb(rusb::Error::Timeout),
            std::io::ErrorKind::BrokenPipe => Error::Usb(rusb::Error::Pipe),
            std::io::ErrorKind::Interrupted => Error::Usb(rusb::Error::Interrupted),
            std::io::ErrorKind::OutOfMemory => Error::Usb(rusb::Error::NoMem),
            std::io::ErrorKind::NotConnected => Error::Nack,
            _ => Error::Usb(rusb::Error::Other),
        }
    }
}
