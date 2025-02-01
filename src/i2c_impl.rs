use crate::{error::*, protocol};
use rusb::{Device, DeviceHandle, GlobalContext, UsbContext};
use std::io::{Read, Write};

pub struct I2c<T: UsbContext> {
    device_handle: DeviceHandle<T>,
    supported_flags: (i2c::ReadFlags, i2c::WriteFlags),
    address: u16,
}

impl<T: UsbContext> I2c<T> {
    #[inline]
    fn open(device: &Device<T>) -> Result<Self> {
        let device_handle = device.open()?;
        device_handle.claim_interface(0)?;
        let supported_flags = protocol::check_device(&device_handle)?;
        Ok(Self {
            device_handle,
            supported_flags,
            address: 0u16,
        })
    }
}

impl I2c<GlobalContext> {
    pub fn open_single_device() -> Result<Self> {
        let devs = crate::devices();
        if devs.is_empty() {
            return Err(rusb::Error::NoDevice.into());
        }
        if devs.len() > 1 {
            return Err(rusb::Error::Other.into());
        }
        I2c::open(&devs[0])
    }
}

impl<T: UsbContext> i2c::Master for I2c<T> {
    type Error = Error;
}

impl<T: UsbContext> i2c::Address for I2c<T> {
    fn set_slave_address(&mut self, addr: u16, tenbit: bool) -> Result<()> {
        if tenbit {
            Err(rusb::Error::NotSupported.into())
        } else {
            self.address = addr;
            Ok(())
        }
    }
}

impl<T: UsbContext> Read for I2c<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        protocol::transfer(
            &self.device_handle,
            &mut [i2c::Message::Read {
                address: self.address,
                data: buf,
                flags: Default::default(),
            }],
        )?;
        Ok(buf.len())
    }
}

impl<T: UsbContext> Write for I2c<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        protocol::transfer(
            &self.device_handle,
            &mut [i2c::Message::Write {
                address: self.address,
                data: buf,
                flags: Default::default(),
            }],
        )?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(()) // noop since no buffering is performed
    }
}

// i2c::ReadWrite should be automatically implemented as long as requirements are met
#[allow(dead_code)]
const fn assert_impl_readwrite<T: i2c::ReadWrite>() {}
const _: () = assert_impl_readwrite::<I2c<GlobalContext>>();

impl<T: UsbContext> i2c::BulkTransfer for I2c<T> {
    fn i2c_transfer_support(&mut self) -> Result<(i2c::ReadFlags, i2c::WriteFlags)> {
        Ok(self.supported_flags)
    }

    fn i2c_transfer(&mut self, messages: &mut [i2c::Message]) -> Result<()> {
        protocol::transfer(&self.device_handle, messages)
    }
}
