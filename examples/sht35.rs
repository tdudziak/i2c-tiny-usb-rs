//! Reads ambient temperature and humidity using a Sensirion SHT35 sensor.
//!
//! ```
//! $ cargo run --example sht35
//!  T = 22.14°C
//! RH = 42.30%
//! ```

use i2c::Address;
use std::io::{Read, Write};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const I2C_ADDR: u16 = 0x44; // can be 0x45 depending on address pins
const CMD_RESET: u16 = 0x30a2;
const CMD_MEASURE_SINGLE: u16 = 0x2400;
const CMD_FETCH_DATA: u16 = 0xe000;

pub fn main() -> Result<()> {
    use std::thread::sleep;
    use std::time::Duration;

    let mut bus = i2c_tiny_usb::I2c::open_single_device()?;
    bus.set_slave_address(I2C_ADDR, false)?;

    // reset the sensor in case it's in a continuous measurement mode
    bus.write_all(&CMD_RESET.to_be_bytes())?;
    sleep(Duration::from_millis(10)); // up to 1.5ms per datasheet

    // trigger a single measurement
    bus.write_all(&CMD_MEASURE_SINGLE.to_be_bytes())?;
    sleep(Duration::from_millis(100)); // up to 15ms per datasheet

    // read the measurement results
    let mut buf = [0u8; 6];
    bus.write_all(&CMD_FETCH_DATA.to_be_bytes())?;
    bus.read_exact(&mut buf)?;

    let temp = u16::from_be_bytes([buf[0], buf[1]]) as f32 * (175.0 / 65565.0) - 45.0;
    let humidity = u16::from_be_bytes([buf[3], buf[4]]) as f32 * (100.0 / 65565.0);
    // the buffer also contains CRCs at buf[2] and buf[5] which are not verified here

    println!(" T = {:.2}°C", temp);
    println!("RH = {:.2}%", humidity);

    Ok(())
}
