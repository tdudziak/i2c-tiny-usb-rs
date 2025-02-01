use i2c::{Message, ReadFlags, WriteFlags};
use std::time::Duration;

use crate::{Connection, Error, Result};

// i2c-tiny-usb and compatible devices can use multiple USB VID+PID combinations
pub(crate) const KNOWN_VENDOR_PRODUCT_IDS: [(u16, u16); 2] = [
    (0x0403, 0xc631), // FTDI
    (0x1c40, 0x0534), // EZPrototypes
];

#[allow(dead_code)]
mod constants {
    pub const CMD_ECHO: u8 = 0;
    pub const CMD_GET_FUNC: u8 = 1;
    pub const CMD_SET_DELAY: u8 = 2;
    pub const CMD_GET_STATUS: u8 = 3;
    pub const CMD_I2C_IO: u8 = 4;

    // following can be OR'd with CMD_I2C_IO
    pub const CMD_I2C_BEGIN: u8 = 1;
    pub const CMD_I2C_END: u8 = 2;

    // possible values for the CMD_GET_STATUS response
    pub const STATUS_IDLE: u8 = 0;
    pub const STATUS_ADDRESS_ACK: u8 = 1;
    pub const STATUS_ADDRESS_NAK: u8 = 2;

    // possible values for the CMD_GET_FUNC response
    pub const I2C_FUNC_I2C: u32 = 0x00000001;
    pub const I2C_FUNC_PROTOCOL_MANGLING: u32 = 0x00000004;

    // per-message flags
    pub const I2C_M_RD: u16 = 0x0001;
    pub const I2C_M_NOSTART: u16 = 0x4000;
    pub const I2C_M_REV_DIR_ADDR: u16 = 0x2000;
    pub const I2C_M_IGNORE_NAK: u16 = 0x1000;
    pub const I2C_M_NO_RD_ACK: u16 = 0x0800;
}
use constants::*;

// control transfer parameters
pub const TIMEOUT: Duration = Duration::from_secs(1);

fn dev_read(
    dev: &impl Connection,
    command: u8,
    flags: ReadFlags,
    arg: u16,
    data: &mut [u8],
) -> Result<()> {
    let mut flag_bits = I2C_M_RD; // needs to be set for all I2C reads
    if flags.contains(ReadFlags::NACK) {
        flag_bits |= I2C_M_NO_RD_ACK;
    }
    if flags.contains(ReadFlags::REVERSE_RW) {
        flag_bits |= I2C_M_REV_DIR_ADDR;
    }
    if flags.contains(ReadFlags::NO_START) {
        flag_bits |= I2C_M_NOSTART;
    }
    let req_type = {
        use rusb::constants::*;
        LIBUSB_REQUEST_TYPE_VENDOR | LIBUSB_RECIPIENT_INTERFACE | LIBUSB_ENDPOINT_IN
    };

    let n_read = dev.read_control(req_type, command, flag_bits, arg, data, TIMEOUT)?;
    if n_read != data.len() {
        Err(rusb::Error::Io.into())
    } else {
        Ok(())
    }
}

fn dev_write(
    dev: &impl Connection,
    command: u8,
    flags: WriteFlags,
    arg: u16,
    data: &[u8],
) -> Result<()> {
    let mut flag_bits = 0;
    if flags.contains(WriteFlags::IGNORE_NACK) {
        flag_bits |= I2C_M_IGNORE_NAK;
    }
    if flags.contains(WriteFlags::REVERSE_RW) {
        flag_bits |= I2C_M_REV_DIR_ADDR;
    }
    if flags.contains(WriteFlags::NO_START) {
        flag_bits |= I2C_M_NOSTART;
    }
    let req_type = {
        use rusb::constants::*;
        LIBUSB_REQUEST_TYPE_VENDOR | LIBUSB_RECIPIENT_INTERFACE | LIBUSB_ENDPOINT_OUT
    };

    let n_written = dev.write_control(req_type, command, flag_bits, arg, data, TIMEOUT)?;
    if n_written != data.len() {
        Err(rusb::Error::Io.into())
    } else {
        Ok(())
    }
}

pub(crate) fn transfer(dev: &impl Connection, messages: &mut [Message]) -> Result<()> {
    if messages.is_empty() {
        return Ok(());
    }
    let i_message_end = messages.len() - 1; // no underflow because of is_empty() check above
    for (i_message, message) in messages.iter_mut().enumerate() {
        let mut cmd = CMD_I2C_IO;
        if i_message == 0 {
            cmd |= CMD_I2C_BEGIN;
        }
        if i_message == i_message_end {
            cmd |= CMD_I2C_END;
        }

        let op_result = match message {
            Message::Read {
                address,
                data,
                flags,
            } => dev_read(dev, cmd, *flags, *address, data),
            Message::Write {
                address,
                data,
                flags,
            } => dev_write(dev, cmd, *flags, *address, data),
        };

        // Typically when there is no acknowledgement, the `op_result` will be a failure because the
        // corresponding USB control transfer is not acknowledged either. We check the status
        // regardless to distinguish this from other errors and in case there are devices that
        // behave differently.
        let mut status: [u8; 1] = [0x0];
        dev_read(dev, CMD_GET_STATUS, ReadFlags::empty(), 0, &mut status)?;
        if status[0] == STATUS_ADDRESS_NAK {
            return Err(Error::Nack);
        }

        // we still want to return an error if there's no NACK but the main operation failed
        op_result?;
    }

    Ok(())
}

/// Issues some test commands and probes the functionality of the i2c-tiny-usb device. Returns
/// supported read and write flags.
pub(crate) fn check_device(dev: &impl Connection) -> Result<(ReadFlags, WriteFlags)> {
    // check the functionality bitmask
    let mut buf_func = [0u8; 4];
    dev_read(dev, CMD_GET_FUNC, ReadFlags::empty(), 0, &mut buf_func)?;
    let func = u32::from_le_bytes(buf_func);
    if func & I2C_FUNC_I2C == 0 {
        // the device doesn't support plain I2C (non-SMBUS) transfers
        return Err(rusb::Error::NotSupported.into());
    }

    // non-standard I2C transfers are only possible if the device supports protocol mangling
    let supported_flags = if func & I2C_FUNC_PROTOCOL_MANGLING != 0 {
        (
            ReadFlags::NACK | ReadFlags::REVERSE_RW | ReadFlags::NO_START,
            WriteFlags::IGNORE_NACK | WriteFlags::REVERSE_RW | WriteFlags::NO_START,
        )
    } else {
        Default::default()
    };

    // test the echo command with a bunch of arbitrary values
    for x in [0u16, 0xaaaa, 0x5555, 0xffff, 0x55aa, 0xaa55, 0x0f0f, 0xf0f0] {
        let mut buf_echo = [0u8; 2];
        let req_type = {
            use rusb::constants::*;
            LIBUSB_REQUEST_TYPE_VENDOR | LIBUSB_RECIPIENT_INTERFACE | LIBUSB_ENDPOINT_IN
        };
        // we cannot use dev_read() for CMD_ECHO since it passes the argument as wValue (normally
        // used for read flags)
        let n_read = dev.read_control(req_type, CMD_ECHO, x, 0, &mut buf_echo, TIMEOUT)?;
        if n_read != 2 || u16::from_le_bytes(buf_echo) != x {
            return Err(rusb::Error::Other.into());
        }
    }

    Ok(supported_flags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::mock::MockConnection;

    #[test]
    fn test_failed_check() {
        let dev = MockConnection::new();
        assert!(check_device(&dev).is_err());
    }

    #[test]
    fn test_check_device() {
        let dev = MockConnection::new();
        dev.schedule_read(
            CMD_GET_FUNC,              // request
            I2C_M_RD,                  // value
            0,                         // index
            &[0x05, 0x00, 0x00, 0x00], // data: I2C + FUNC_PROTOCOL_MANGLING
        );
        for x in [0u16, 0xaaaa, 0x5555, 0xffff, 0x55aa, 0xaa55, 0x0f0f, 0xf0f0] {
            dev.schedule_read(
                CMD_ECHO,         // request
                x,                // value
                0,                // index
                &x.to_le_bytes(), // data
            );
        }
        let (read_flags, write_flags) = check_device(&dev).unwrap();
        assert!(read_flags.contains(ReadFlags::NACK));
        assert!(read_flags.contains(ReadFlags::REVERSE_RW));
        assert!(read_flags.contains(ReadFlags::NO_START));
        assert!(write_flags.contains(WriteFlags::IGNORE_NACK));
        assert!(write_flags.contains(WriteFlags::REVERSE_RW));
        assert!(write_flags.contains(WriteFlags::NO_START));
    }

    #[test]
    fn test_transfer_zero_length() {
        let dev = MockConnection::new();
        let mut msgs: [Message; 0] = [];
        transfer(&dev, &mut msgs).unwrap();
        assert!(!dev.has_writes(), "no write I2C transactions expected");
    }

    #[test]
    fn test_transfer_single_write() {
        let dev = MockConnection::new();
        dev.schedule_read(CMD_GET_STATUS, I2C_M_RD, 0, &[STATUS_IDLE]);
        let mut msgs = [Message::Write {
            address: 0x50,
            data: &[0x11, 0x22],
            flags: WriteFlags::empty(),
        }];

        transfer(&dev, &mut msgs).unwrap();
        assert!(
            dev.pop_write(
                CMD_I2C_IO | CMD_I2C_BEGIN | CMD_I2C_END,
                0,    // flags for WriteFlags::empty()
                0x50, // address
                &[0x11, 0x22]
            ),
            "expected write transaction was not found"
        );
        assert!(!dev.has_writes(), "no more write I2C transactions expected");
    }

    #[test]
    fn test_transfer_single_read() {
        let dev = MockConnection::new();
        dev.schedule_read(
            CMD_I2C_IO | CMD_I2C_BEGIN | CMD_I2C_END,
            I2C_M_RD,
            0x50,
            &[0xAA, 0xBB, 0xCC],
        );
        dev.schedule_read(CMD_GET_STATUS, I2C_M_RD, 0, &[STATUS_IDLE]);

        let mut read_buf = [0u8; 3];
        let mut msgs = [Message::Read {
            address: 0x50,
            data: &mut read_buf,
            flags: ReadFlags::empty(),
        }];
        transfer(&dev, &mut msgs).unwrap();
        assert!(read_buf == [0xAA, 0xBB, 0xCC]);
        assert!(!dev.has_writes(), "no write I2C transactions expected");
    }

    #[test]
    fn test_transfer_two_messages() {
        let dev = MockConnection::new();

        // response and status for the read message
        dev.schedule_read(5, I2C_M_RD, 0x10, &[0x01, 0x02]);
        dev.schedule_read(CMD_GET_STATUS, I2C_M_RD, 0, &[STATUS_IDLE]);

        // status for the write message
        dev.schedule_read(CMD_GET_STATUS, I2C_M_RD, 0, &[STATUS_IDLE]);

        let mut read_buf = [0u8; 2];
        let mut msgs = [
            Message::Read {
                address: 0x10,
                data: &mut read_buf,
                flags: ReadFlags::empty(),
            },
            Message::Write {
                address: 0x20,
                data: &[0xAA, 0xBB, 0xCC],
                flags: WriteFlags::empty(),
            },
        ];

        transfer(&dev, &mut msgs).unwrap();
        assert_eq!(read_buf, [0x01, 0x02]);
        assert!(dev.pop_write(
            CMD_I2C_IO | CMD_I2C_END, // = 6
            0,                        // WriteFlags::empty
            0x20,                     // address
            &[0xAA, 0xBB, 0xCC]       // data
        ));
        assert!(!dev.has_writes(), "no more write I2C transactions expected");
    }
}
