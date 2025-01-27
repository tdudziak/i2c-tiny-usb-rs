mod error;
mod i2c_impl;
mod protocol;
mod connection;

#[cfg(all(test, feature = "hw-tests"))]
mod hw_tests;

pub(crate) use connection::Connection;

pub use error::*;
pub use i2c_impl::*;
pub use protocol::{ID_PRODUCT, ID_VENDOR};
pub use rusb;
