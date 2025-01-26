use std::time::Duration;

use rusb::{DeviceHandle, UsbContext};

/// Trait used in `crate::protocol` to communicate with the USB device. Provides only control
/// transfers since this is what the i2c-tiny-usb protocol uses. Can be replaced with
/// `MockConnection` for testing.
pub(crate) trait Connection {
    fn read_control(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        buf: &mut [u8],
        timeout: Duration,
    ) -> rusb::Result<usize>;

    fn write_control(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        buf: &[u8],
        timeout: Duration,
    ) -> rusb::Result<usize>;
}

impl<T: UsbContext> Connection for DeviceHandle<T> {
    #[inline]
    fn read_control(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        buf: &mut [u8],
        timeout: Duration,
    ) -> rusb::Result<usize> {
        self.read_control(request_type, request, value, index, buf, timeout)
    }

    #[inline]
    fn write_control(
        &self,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        buf: &[u8],
        timeout: Duration,
    ) -> rusb::Result<usize> {
        self.write_control(request_type, request, value, index, buf, timeout)
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[derive(Debug, Default, Clone)]
    pub struct Transaction {
        pub request: u8,
        pub value: u16,
        pub index: u16,
        pub data: Vec<u8>,
    }

    pub struct MockConnection {
        pub last_writes: RefCell<Vec<Transaction>>,
        pub next_reads: RefCell<VecDeque<Transaction>>,
    }

    impl Connection for MockConnection {
        fn read_control(
            &self,
            _request_type: u8,
            request: u8,
            value: u16,
            index: u16,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> rusb::Result<usize> {
            let mut next_reads = self.next_reads.borrow_mut();
            let t = match next_reads.pop_front() {
                None => return Err(rusb::Error::Io),
                Some(x) => x,
            };
            if t.request != request
                || t.value != value
                || t.index != index
                || t.data.len() != buf.len()
            {
                // the read request doesn't match the scheduled response
                return Err(rusb::Error::Io);
            }
            buf.copy_from_slice(&t.data);
            Ok(buf.len())
        }

        fn write_control(
            &self,
            _request_type: u8,
            request: u8,
            value: u16,
            index: u16,
            buf: &[u8],
            _timeout: Duration,
        ) -> rusb::Result<usize> {
            let mut writes = self.last_writes.borrow_mut();
            writes.push(Transaction {
                request,
                value,
                index,
                data: buf.into(),
            });
            Ok(buf.len())
        }
    }

    impl MockConnection {
        pub fn new() -> Self {
            Self {
                last_writes: RefCell::new(Vec::new()),
                next_reads: RefCell::new(VecDeque::new()),
            }
        }

        pub fn schedule_read(&self, request: u8, value: u16, index: u16, data: &[u8]) {
            self.next_reads.borrow_mut().push_back(Transaction {
                request,
                value,
                index,
                data: data.into(),
            });
        }

        pub fn pop_write(&self, request: u8, value: u16, index: u16, data: &[u8]) -> bool {
            let mut writes = self.last_writes.borrow_mut();
            match writes.pop() {
                None => false,
                Some(t) => {
                    t.request == request && t.value == value && t.index == index && t.data == data
                }
            }
        }

        pub fn has_writes(&self) -> bool {
            !self.last_writes.borrow().is_empty()
        }
    }
}
