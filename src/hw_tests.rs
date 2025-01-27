//! This module contains automated testcases that require a system with a connected i2c-tiny-usb
//! device so they're not run by default. If you want to include them, run the tests with:
//! `cargo test --features hw-tests`

use crate::I2c;

#[test]
pub fn test_connect() {
    I2c::open_single_device().unwrap();
}
