//! This module contains automated testcases that require a system with a connected i2c-tiny-usb
//! device so they're not run by default. If you want to include them, run the tests with:
//! `cargo test --features hw-tests`
//!
//! Assumptions made about the hardware setup:
//! - There is exactly one i2c-tiny-usb device connected.
//! - There is at least one peripheral on the bus that acknowledges a general call (read at 0x00).
//! - No peripheral has the (reserved) address 0x03.
//! - There is an EEPROM chip at address 0x50 with the test pattern `EEPROM_TEST_PATTERN` programmed
//!   at offset 0x20. You can enable the additional feature `hw-tests-program-eeprom` to write to the
//!   EEPROM prior to checking the test pattern.

use i2c::{Address, BulkTransfer};
use serial_test::serial;
use std::io::{Read, Write};

use crate::{Error, I2c};

/// Only connects to the device and initializes the interface. Internally, [`crate::I2c`] will read
/// the functionality and perform an echo test.
#[test]
#[serial(device)]
pub fn test_connect() {
    I2c::open_single_device().unwrap();
}

/// Attempts a read and a write on the reserved 0x03 address using the BulkTransfer interface and
/// makes sure that the result is reported as a NACK.
#[test]
#[serial(device)]
pub fn test_nack() {
    use i2c::Message;
    let mut bus = I2c::open_single_device().unwrap();

    let mut read_buf = [0u8; 2];
    let read_res = bus.i2c_transfer(&mut [Message::Read {
        address: 0x03,
        data: &mut read_buf,
        flags: Default::default(),
    }]);
    assert_eq!(read_res, Err(Error::Nack));

    let write_res = bus.i2c_transfer(&mut [Message::Write {
        address: 0x03,
        data: &[0, 1, 2, 3],
        flags: Default::default(),
    }]);
    assert_eq!(write_res, Err(Error::Nack));
}

/// Attempts a read and a write on the reserved 0x03 address using the std::io interfaces and makes
/// sure the transactions are not acknowledged.
#[test]
#[serial(device)]
pub fn test_nack_std_io() {
    let mut bus = I2c::open_single_device().unwrap();
    bus.set_slave_address(0x03, false).unwrap();

    let mut read_buf = [0u8; 2];
    let read_res = bus.read(&mut read_buf);
    assert_eq!(
        read_res.err().unwrap().kind(),
        std::io::ErrorKind::NotConnected
    );

    let write_res = bus.write(&[0, 1, 2, 3]);
    assert_eq!(
        write_res.err().unwrap().kind(),
        std::io::ErrorKind::NotConnected
    );
}

/// Issues a "general call" read with both the BulkTransfer and std::io::Read interfaces and makes
/// sure that it's acknowledged.
#[test]
#[serial(device)]
pub fn test_general_call() {
    use i2c::Message;
    let mut bus = I2c::open_single_device().unwrap();
    let mut read_buf = [];

    // test using the BulkTransfer interface
    let read_res = bus.i2c_transfer(&mut [Message::Read {
        address: 0x00,
        data: &mut read_buf,
        flags: Default::default(),
    }]);
    assert!(read_res.is_ok());

    // test using std::io::Read interface
    bus.set_slave_address(0x00, false).unwrap();
    let mut read_buf = [];
    let read_res = bus.read(&mut read_buf);
    assert!(read_res.is_ok());
}

// test pattern expected in the EEPROM at address 0x20
const EEPROM_TEST_PATTERN: [u8; 16] = [
    0xaa, 0x55, 0xa0, 0xa5, 0x0a, 0x5a, 0xff, 0x00, 0x12, 0xca, 0xff, 0xee, 0x12, 0x23, 0x34, 0x45,
];

#[cfg(feature = "hw-tests-program-eeprom")]
fn pre_program_eeprom(bus: &mut I2c<impl rusb::UsbContext>) {
    use i2c::Message;

    let mut data = [0u8; 17];
    data[0] = 0x20;
    data[1..].copy_from_slice(&EEPROM_TEST_PATTERN);

    bus.i2c_transfer(&mut [Message::Write {
        address: 0x50,
        data: &data,
        flags: Default::default(),
    }])
    .unwrap();
}

#[test]
#[serial(device)]
pub fn test_eeprom_read() {
    use i2c::Message;
    let mut bus = I2c::open_single_device().unwrap();

    #[cfg(feature = "hw-tests-program-eeprom")]
    pre_program_eeprom(&mut bus);

    // we read 0x20 (32) bytes twice each time reading 8 bytes before and after the test pattern
    let mut read_buf = [0u8; 0x40];
    for off in &[0x00, 0x20] {
        bus.i2c_transfer(&mut [
            Message::Write {
                address: 0x50,
                data: &[0x20 - 8],
                flags: Default::default(),
            },
            Message::Read {
                address: 0x50,
                data: &mut read_buf[*off..*off + 0x20],
                flags: Default::default(),
            },
        ])
        .unwrap();
    }

    // test pattern at offset 8 in both reads
    assert_eq!(&read_buf[8..24], &EEPROM_TEST_PATTERN);
    assert_eq!(&read_buf[0x20 + 8..0x20 + 24], &EEPROM_TEST_PATTERN);

    // both reads should have read the same memory (outside of the pattern as well)
    assert_eq!(&read_buf[..0x20], &read_buf[0x20..]);
}

#[test]
#[serial(device)]
#[ignore] // FIXME: currently fails in a way that breaks other tests
pub fn test_error_recovery() {
    fn check_valid_read(bus: &mut I2c<impl rusb::UsbContext>) {
        let mut buf = [0u8; 16];
        bus.write_all(&[0x20]).unwrap();
        bus.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, &EEPROM_TEST_PATTERN);
    }

    let mut bus = I2c::open_single_device().unwrap();
    {
        #[cfg(feature = "hw-tests-program-eeprom")]
        pre_program_eeprom(&mut bus);
    }
    bus.set_slave_address(0x50, false).unwrap();
    check_valid_read(&mut bus);

    // check that everything still works after a NACK
    bus.set_slave_address(0x03, false).unwrap();
    assert!(bus.write_all(&[0]).is_err());
    bus.set_slave_address(0x50, false).unwrap();
    check_valid_read(&mut bus);

    // reads of 128 bytes or more seem to fail
    let mut big_buf = [0u8; 256];
    bus.write_all(&[0u8]).unwrap();
    assert!(bus.read_exact(&mut big_buf[..128]).is_err());
    check_valid_read(&mut bus);
    assert!(bus.read_exact(&mut big_buf[..256]).is_err());
    check_valid_read(&mut bus);
}

#[test]
#[serial(device)]
pub fn test_eeprom_big_reads() {
    let mut bus = I2c::open_single_device().unwrap();
    let mut buf = [0u8; 256];

    #[cfg(feature = "hw-tests-program-eeprom")]
    pre_program_eeprom(&mut bus);

    // FIXME: on macOS with a i2c-star firmware device, reads of 102 or bigger time out and reads
    // 128 or bigger fail immediately. Afterwards, the device gets stuck in an error state and
    // requires cable unplug/replug. Is this a firmware/library bug or our problem?
    const MAX_READ: usize = 101;

    for size in 16..=MAX_READ {
        bus.set_slave_address(0x50, false).unwrap();
        bus.write_all(&[0x20]).unwrap();
        bus.read_exact(&mut buf[..size]).unwrap();
        assert_eq!(&buf[..16], &EEPROM_TEST_PATTERN);
    }
}
