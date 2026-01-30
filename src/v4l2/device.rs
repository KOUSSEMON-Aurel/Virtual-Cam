use std::io;
use crate::v4l2::dmabuf::DmaBufAllocator;
use std::os::unix::io::RawFd;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

pub struct Device {
    fd: RawFd,
    allocator: DmaBufAllocator,
}

impl Device {
    pub fn open(nr: u16) -> io::Result<Self> {
        let path = format!("/dev/video{}", nr);
        
        // Ouverture réelle du périphérique en écriture seule
        let file = OpenOptions::new()
            .read(false)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&path)?;
        
        use std::os::unix::io::IntoRawFd;
        let fd = file.into_raw_fd();
        
        let allocator = DmaBufAllocator::new(&path, 1920 * 1080 * 2)?; 
        
        Ok(Self {
            fd,
            allocator,
        })
    }
    
    pub fn write_frame_dmabuf(&self, data: &[u8]) -> io::Result<()> {
        // Envoi réellement les données au kernel via l'allocateur DMA-BUF
        self.allocator.write_frame(data)
    }
}
