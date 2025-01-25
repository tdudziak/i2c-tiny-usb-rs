use i2c::{Message, ReadFlags, WriteFlags};
use rusb::*;
use std::time::Duration;

use crate::{Error, Result};

#[allow(dead_code)]
mod constants {
    pub const ID_VENDOR: u16 = 0x0403;
    pub const ID_PRODUCT: u16 = 0xc631;

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
pub use constants::{ID_PRODUCT, ID_VENDOR};

// control transfer parameters
pub const TIMEOUT: Duration = Duration::from_secs(1);
pub const REQ_TYPE: u8 =
    rusb::constants::LIBUSB_REQUEST_TYPE_VENDOR | rusb::constants::LIBUSB_RECIPIENT_INTERFACE;

fn encode_read_flags(flags: ReadFlags) -> u16 {
    let mut result = I2C_M_RD; // needs to be set for all I2C reads
    if flags.contains(ReadFlags::NACK) {
        result |= I2C_M_NO_RD_ACK;
    }
    if flags.contains(ReadFlags::REVERSE_RW) {
        result |= I2C_M_REV_DIR_ADDR;
    }
    if flags.contains(ReadFlags::NO_START) {
        result |= I2C_M_NOSTART;
    }
    result
}

fn encode_write_flags(flags: WriteFlags) -> u16 {
    let mut result = 0;
    if flags.contains(WriteFlags::IGNORE_NACK) {
        result |= I2C_M_IGNORE_NAK;
    }
    if flags.contains(WriteFlags::REVERSE_RW) {
        result |= I2C_M_REV_DIR_ADDR;
    }
    if flags.contains(WriteFlags::NO_START) {
        result |= I2C_M_NOSTART;
    }
    result
}

pub(crate) fn transfer<T: UsbContext>(
    dev: &DeviceHandle<T>,
    messages: &mut [Message],
) -> Result<()> {
    let i_message_end = messages.len() - 1;
    for (i_message, message) in messages.iter_mut().enumerate() {
        let mut cmd = CMD_I2C_IO;
        if i_message == 0 {
            cmd |= CMD_I2C_BEGIN;
        }
        if i_message == i_message_end {
            cmd |= CMD_I2C_END;
        }

        match message {
            Message::Read {
                address,
                data,
                flags,
            } => {
                let flag_bits = encode_read_flags(*flags);
                dev.read_control(REQ_TYPE, cmd, flag_bits, *address, data, TIMEOUT)?;
            }
            Message::Write {
                address,
                data,
                flags,
            } => {
                let flag_bits = encode_write_flags(*flags);
                dev.write_control(REQ_TYPE, cmd, flag_bits, *address, data, TIMEOUT)?;
            }
        }

        // read status and check for NACK condition
        let mut status: [u8; 1] = [0x0];
        dev.read_control(REQ_TYPE, CMD_GET_STATUS, 0, 0, &mut status, TIMEOUT)?;
        if status[0] == STATUS_ADDRESS_NAK {
            return Err(Error::Nack);
        }
    }

    todo!();
}

/// Issues some test commands and probes the functionality of the i2c-tiny-usb device. Returns
/// supported read and write flags.
pub(crate) fn check_device<T: UsbContext>(
    dev: &DeviceHandle<T>,
) -> Result<(ReadFlags, WriteFlags)> {
    // check the functionality bitmask
    let mut buf_func = [0u8; 4];
    dev.read_control(REQ_TYPE, CMD_GET_FUNC, 0, 0, &mut buf_func, TIMEOUT)?;
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
        dev.read_control(REQ_TYPE, CMD_ECHO, 0, x, &mut buf_echo, TIMEOUT)?;
        if buf_echo != x.to_le_bytes() {
            return Err(rusb::Error::Other.into());
        }
    }

    Ok(supported_flags)
}
