mod report_descriptor;

use log;

use tokio::{io, io::AsyncRead, io::AsyncWrite, io::PollEvented};

use std::ffi::CString;
use std::{
    io::{Read, Write},
    os::unix::io::RawFd,
    pin::Pin,
    task::Context,
};

mod ffi {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

mod unix;

use ffi::hidraw_report_descriptor;
pub use report_descriptor::{RawReport, ReportItem};

use nix::{
    errno::Errno,
    ioctl_read,
    libc::{c_int, close, open, O_NONBLOCK, O_RDWR},
    unistd::{read, write},
};

pub use unix::list_devices;

use mio::{unix::EventedFd, Evented, Poll, PollOpt, Ready, Token};

use anyhow::Result;

ioctl_read!(read_desc_size, b'H', 0x01, c_int);

ioctl_read!(read_report_descriptor, b'H', 0x02, hidraw_report_descriptor);

type HidrawReportDescriptor = ffi::hidraw_report_descriptor;

pub struct Device {
    shared: PollEvented<SharedState>,
}

impl Device {
    // This should not be available
    unsafe fn fd(&self) -> RawFd {
        self.shared.get_ref().fd
    }

    pub fn feature_report(&self) -> Result<RawReport> {
        let desc_size = unsafe { self.report_descriptor_size()? };

        let raw_descriptor = unsafe { self.read_report_descriptor(desc_size as u32)? };

        Ok(RawReport(Vec::from(
            &raw_descriptor.value[..(desc_size as usize)],
        )))
    }

    unsafe fn report_descriptor_size(&self) -> Result<i32> {
        let mut desc_size = 0;
        read_desc_size(self.fd(), &mut desc_size)?;

        Ok(desc_size)
    }

    unsafe fn read_report_descriptor(&self, size: u32) -> Result<HidrawReportDescriptor> {
        let mut report_descriptor = hidraw_report_descriptor {
            size,
            value: [0u8; 4096],
        };

        read_report_descriptor(self.fd(), &mut report_descriptor)?;

        Ok(report_descriptor)
    }
}

impl AsyncRead for Device {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().shared).poll_read(cx, buf)
    }
}

impl AsyncWrite for Device {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().shared).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().shared).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> std::task::Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().shared).poll_shutdown(cx)
    }
}

struct SharedState {
    fd: RawFd,
}

impl Evented for SharedState {
    fn register(
        &self,
        registry: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.fd).register(registry, token, interest, opts)
    }

    fn reregister(
        &self,
        registry: &Poll,
        token: Token,
        interests: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.fd).reregister(registry, token, interests, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.fd).deregister(poll)
    }
}

/*
impl AsyncRead for Device {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let read_result = read(self.fd, buf);

        match read_result {
            Ok(n) if n == 0 => {
                let waker = cx.waker();

                // Todo: register this with context
                Poll::Pending
            }
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => Poll::Ready(Err(io::Error::from_raw_os_error(
                e.as_errno().unwrap() as i32
            ))),
        }
    }
}
*/

impl Device {
    fn new(fd: RawFd) -> Device {
        let shared = SharedState { fd };

        let poll_evented = PollEvented::new(shared).unwrap();

        Device {
            shared: poll_evented,
        }
    }

    pub fn from_path(path: &str) -> anyhow::Result<Self> {
        let path = CString::new(path)?;

        let fd = unsafe { Errno::result(open(path.as_ptr(), O_RDWR | O_NONBLOCK))? };

        Ok(Device::new(fd))
    }

    /*
    pub unsafe fn fd(&self) -> c_int {
        self.fd
    }

    pub fn read(&self, data: &mut [u8]) -> anyhow::Result<usize> {
        let bytes_read = read(self.fd, data)?;

        Ok(bytes_read)
    }

    pub fn write(&self, data: &[u8]) -> anyhow::Result<usize> {
        let bytes_written = write(self.fd, data)?;

        Ok(bytes_written)
    }
    */
}

impl Read for SharedState {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes = read(self.fd, buf).map_err(|err| {
            let errno = err.as_errno().unwrap();
            io::Error::from_raw_os_error(errno as i32)
        })?;

        Ok(bytes)
    }
}

impl Write for SharedState {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let bytes = write(self.fd, buf).map_err(|err| {
            let errno = err.as_errno().unwrap();
            io::Error::from_raw_os_error(errno as i32)
        })?;

        Ok(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // Figure out how to do this
        unimplemented!();
        /*
        let bytes = flush(self.fd).map_err(|err| {
            let errno = err.as_errno().unwrap();
            io::Error::from_raw_os_error(errno as i32)
        })?;

        Ok(bytes)
        */
    }
}

impl Drop for SharedState {
    fn drop(&mut self) {
        unsafe {
            let result = Errno::result(close(self.fd));

            if let Err(e) = result {
                log::warn!("Error closing device: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
