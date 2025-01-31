//! Attempts a read transaction on every allowed peripheral address and prints results in a format
//! similar to the standard `i2cdetect` command line tool.
//!
//! ```
//! $ cargo run --example i2cdetect
//!      0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f
//! 00:          -- -- -- -- -- -- -- -- -- -- -- -- --
//! 10: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --
//! 20: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --
//! 30: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --
//! 40: -- -- -- -- 44 -- -- -- -- -- -- -- -- -- -- --
//! 50: 50 -- -- -- -- -- -- -- -- -- -- -- -- -- -- --
//! 60: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --
//! 70: -- -- -- -- -- -- -- --
//! ```

use i2c::{BulkTransfer, Message};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn main() -> Result<()> {
    let mut bus = i2c_tiny_usb::I2c::open_single_device()?;

    println!("     0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f");
    print!("00:          ");
    for address in 0x03..=0x77 {
        // attempt a single zero-length read
        let result = bus.i2c_transfer(&mut [Message::Read {
            address,
            data: &mut [],
            flags: Default::default(),
        }]);

        match result {
            Ok(()) => print!("{:02x}", address),
            Err(i2c_tiny_usb::Error::Nack) => print!("--"),
            Err(_) => print!("EE"),
        }
        if address & 0x0f == 0x0f {
            println!();
            print!("{:02x}: ", address + 1);
        } else {
            print!(" ");
        }
    }
    println!();

    Ok(())
}
