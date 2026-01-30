use io_uring::{opcode, types, IoUring};
use std::io;
use std::net::UdpSocket;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use crate::memory::pool::BufferPool;

pub struct UringReceiver {
    ring: IoUring,
    socket: UdpSocket,
    buffer_pool: Arc<BufferPool>,
}

impl UringReceiver {
    pub fn new(port: u16, buffer_pool: Arc<BufferPool>) -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))?;
        socket.set_nonblocking(true)?;
        
        // Configuration agressive de l'io_uring pour minimiser la latence
        let ring = IoUring::builder()
            .setup_sqpoll(2000) // Kernel polling pour éviter les syscalls d'entrée
            .build(4096)?;
            
        Ok(Self {
            ring,
            socket,
            buffer_pool,
        })
    }
    
    pub async fn recv(&mut self) -> io::Result<Vec<u8>> {
        let fd = self.socket.as_raw_fd();
        let mut buffer = self.buffer_pool.acquire().await;
        
        let recv_e = opcode::Recv::new(
            types::Fd(fd),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        );
        
        unsafe {
            self.ring
                .submission()
                .push(&recv_e.build().user_data(0x42))?;
        }
        
        self.ring.submit_and_wait(1)?;
        
        let cqe = self.ring.completion().next().expect("CQE missing");
        let bytes_read = cqe.result();
        
        if bytes_read < 0 {
            return Err(io::Error::from_raw_os_error(-bytes_read));
        }
        
        buffer.truncate(bytes_read as usize);
        Ok(buffer)
    }
}
