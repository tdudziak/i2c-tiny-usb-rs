use i2c::Message;
use rusb::*;
use std::time::Duration;

use crate::Result;

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
}
use constants::*;
pub use constants::{ID_PRODUCT, ID_VENDOR};

// control transfer parameters
pub const TIMEOUT: Duration = Duration::from_secs(1);
pub const REQ_TYPE: u8 =
    rusb::constants::LIBUSB_REQUEST_TYPE_VENDOR | rusb::constants::LIBUSB_RECIPIENT_INTERFACE;

pub(crate) fn transfer<T: UsbContext>(
    dev: &DeviceHandle<T>,
    messages: &mut [Message],
) -> Result<()> {
    let flags = 0; // TODO: should this ever be anything else?
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
            Message::Read { address, data, .. } => {
                dev.read_control(REQ_TYPE, cmd, flags, *address, data, TIMEOUT)?;
            }
            Message::Write { address, data, .. } => {
                dev.write_control(REQ_TYPE, cmd, flags, *address, data, TIMEOUT)?;
            }
        }

        // read status and check for NACK condition
        let mut status: [u8; 1] = [0x0];
        dev.read_control(REQ_TYPE, CMD_GET_STATUS, 0, 0, &mut status, TIMEOUT)?;
        if status[0] == STATUS_ADDRESS_NAK {
            // TODO: better error for NACK?
            return Err(rusb::Error::NoDevice.into());
        }
    }

    todo!();
}

pub(crate) fn check_device<T: UsbContext>(dev: &DeviceHandle<T>) -> Result<()> {
    // check the functionality bitmask; at the very least the device should support I2C
    let mut buf_func = [0u8; 4];
    dev.read_control(REQ_TYPE, CMD_GET_FUNC, 0, 0, &mut buf_func, TIMEOUT)?;
    if buf_func != I2C_FUNC_I2C.to_le_bytes() {
        return Err(rusb::Error::NotSupported.into());
    }

    // test the echo command with a bunch of arbitrary values
    for x in [0u16, 0xaaaa, 0x5555, 0xffff, 0x55aa, 0xaa55, 0x0f0f, 0xf0f0] {
        let mut buf_echo = [0u8; 2];
        dev.read_control(REQ_TYPE, CMD_ECHO, 0, x, &mut buf_echo, TIMEOUT)?;
        if buf_echo != x.to_le_bytes() {
            return Err(rusb::Error::Other.into());
        }
    }

    Ok(())
}
