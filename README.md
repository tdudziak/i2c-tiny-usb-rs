# i2c-tiny-usb

A Rust library for communicating with USB-to-I2C adapters from userspace using `libusb` via the
`rusb` crate.  The library is compatible with a variety of USB adapters supported by the
`i2c-tiny-usb` Linux kernel driver.

The library implements traits from the [i2c](https://crates.io/crates/i2c) crate so it can be used
as a drop-in replacement for `i2c-linux` on non-Linux platforms if no other implementation is
available.

## Examples

You can find some example programs using the library in the `examples/` directory. Most of them
require some kind of extra hardware to be present on the I2C bus which is described in the top level
comment of each example.

- `dump-eeprom.rs`: Dumps content from an I2C EEPROM
- `i2cdetect.rs`: Scans for devices on the I2C bus
- `sht35.rs`: Reads temperature and humidity from an SHT35 sensor

## Hardware Tests

Basic testcases can be simply run with `cargo test` but other tests require a pysical setup with an
`i2c-tiny-usb` device present and an EEPROM chip connected to the I2C bus. Check the top level
comment of `hw_tests.rs` for more details.

Run the hardware tests with:

```bash
cargo test --features hw-tests
```
