use std::os::unix::io::RawFd;
use std::{io, ptr};
use std::ffi::CString;

pub struct DmaBufAllocator {
    fd: RawFd,
    size: usize,
    addr: *mut u8,
}

unsafe impl Send for DmaBufAllocator {}
unsafe impl Sync for DmaBufAllocator {}

impl DmaBufAllocator {
    pub fn new(device: &str, size: usize) -> io::Result<Self> {
        let path = CString::new(device).unwrap();
        
        let fd = unsafe {
            libc::open(path.as_ptr(), libc::O_RDWR)
        };
        
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        
        // Memory map (zero-copy bypass)
        let addr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        
        if addr == libc::MAP_FAILED {
            unsafe { libc::close(fd); }
            return Err(io::Error::last_os_error());
        }
        
        Ok(Self { fd, size, addr: addr as *mut u8 })
    }
    
    pub fn write_frame(&self, data: &[u8]) -> io::Result<()> {
        unsafe {
            // Copie directe dans la mémoire mappée (accessible par le DMA du kernel)
            ptr::copy_nonoverlapping(data.as_ptr(), self.addr, data.len().min(self.size));
        }
        Ok(())
    }
}

impl Drop for DmaBufAllocator {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.addr as *mut _, self.size);
            libc::close(self.fd);
        }
    }
}
