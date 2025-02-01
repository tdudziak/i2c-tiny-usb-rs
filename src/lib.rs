mod connection;
mod error;
mod i2c_impl;
mod protocol;

#[cfg(all(test, feature = "hw-tests"))]
mod hw_tests;

pub(crate) use connection::Connection;

pub use error::*;
pub use i2c_impl::*;
pub use rusb;

use rusb::{Device, GlobalContext, UsbContext};

pub fn is_supported_device<T: UsbContext>(dev: &Device<T>) -> bool {
    let desc = match dev.device_descriptor() {
        Err(_) => return false,
        Ok(x) => x,
    };
    for (vid, pid) in protocol::KNOWN_VENDOR_PRODUCT_IDS {
        if desc.vendor_id() == vid && desc.product_id() == pid {
            return true;
        }
    }
    false
}

pub fn devices() -> Vec<Device<GlobalContext>> {
    match rusb::devices() {
        Err(_) => vec![],
        Ok(devs) => devs.iter().filter(is_supported_device).collect(),
    }
}
