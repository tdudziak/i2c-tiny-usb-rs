use i2c::Message;
use rusb::*;

use crate::Result;

pub const ID_VENDOR: u16 = 0x0403;
pub const ID_PRODUCT: u16 = 0xc631;

const CMD_ECHO: u8 = 0;
const CMD_GET_FUNC: u8 = 1;
const CMD_SET_DELAY: u8 = 2;
const CMD_GET_STATUS: u8 = 3;
const CMD_I2C_IO: u8 = 4;

// following can be OR'd with CMD_I2C_IO
const CMD_I2C_BEGIN: u8 = 1;
const CMD_I2C_END: u8 = 2;

pub(crate) fn transfer<T: UsbContext>(dev: &DeviceHandle<T>, messages: &[Message]) -> Result<()> {
    todo!();
}
