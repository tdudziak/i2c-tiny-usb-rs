//! Dumps the contents of an EEPROM chip at I2C address 0x50. Pass the number of bytes (e.g. the
//! EEPROM size) as command line argument.
//!
//! ```
//! cargo run --example dump-eeprom 64
//! 0000  ff ff ff ff ff ff ff ff  ff ff ff ff ff ff ff ff   |................|
//! 0010  ff ff ff ff ff ff ff ff  ff ff ff ff ff ff ff ff   |................|
//! 0020  aa 55 a0 a5 0a 5a ff 00  12 ca ff ee 12 23 34 45   |.U...Z.......#4E|
//! 0030  ff ff ff ff ff ff ff ff  ff ff ff ff ff ff ff ff   |................|
//! ```

use i2c::{BulkTransfer, Message};
use std::error::Error;

const EEPROM_ADDR: u16 = 0x50;
const BLOCK_SIZE: usize = 16;

fn main() -> Result<(), Box<dyn Error>> {
    let bytes_to_read = match std::env::args().nth(1) {
        Some(val) => val.parse::<u16>()?,
        None => return Err("Missing argument".into()),
    };

    let mut bus = i2c_tiny_usb::I2c::open_single_device()?;
    let mut offset: u16 = 0;
    while offset < bytes_to_read {
        let mut read_buf = [0u8; BLOCK_SIZE];

        // we use single-byte addressing if possible since smaller chips might support only it
        let addr_bytes16 = offset.to_be_bytes();
        let addr_bytes = if addr_bytes16[0] == 0 {
            &addr_bytes16[1..]
        } else {
            &addr_bytes16
        };

        bus.i2c_transfer(&mut [
            Message::Write {
                address: EEPROM_ADDR,
                data: addr_bytes,
                flags: Default::default(),
            },
            Message::Read {
                address: EEPROM_ADDR,
                data: &mut read_buf,
                flags: Default::default(),
            },
        ])?;
        print_hexdump_line(offset as u32, &read_buf);

        offset += BLOCK_SIZE as u16;
    }
    Ok(())
}

/// Prints one 16-byte line in a style similar to `hexdump -C`.
fn print_hexdump_line(base_offset: u32, data: &[u8]) {
    assert!(data.len() == BLOCK_SIZE);
    print!("{:04x}  ", base_offset);

    // hex bytes split into two 8-byte columns
    for (i, byte) in data.iter().enumerate() {
        if i == 8 {
            print!(" ");
        }
        print!("{:02x} ", byte);
    }

    // print printable characters in a separate column between `|`
    print!("  |");
    for byte in data.iter() {
        let c = if byte.is_ascii_graphic() || *byte == b' ' {
            *byte as char
        } else {
            '.'
        };
        print!("{}", c);
    }
    println!("|");
}
