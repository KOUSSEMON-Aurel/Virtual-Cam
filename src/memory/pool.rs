use crossbeam::queue::ArrayQueue;
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct BufferPool {
    queue: ArrayQueue<Vec<u8>>,
    semaphore: Semaphore,
    buffer_capacity: usize,
}

impl BufferPool {
    pub fn new(count: usize, capacity: usize) -> Self {
        let queue = ArrayQueue::new(count);
        for _ in 0..count {
            queue.push(vec![0u8; capacity]).unwrap();
        }
        
        Self {
            queue,
            semaphore: Semaphore::new(count),
            buffer_capacity: capacity,
        }
    }
    
    pub async fn acquire(&self) -> Vec<u8> {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.queue.pop().expect("Buffer missing despite permit")
    }
    
    pub fn release(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        // Resize back to original capacity if it was shrunk, or keep if it's the same
        if buffer.capacity() < self.buffer_capacity {
             buffer.reserve(self.buffer_capacity);
        }
        unsafe { buffer.set_len(self.buffer_capacity); }
        
        self.queue.push(buffer).unwrap();
        self.semaphore.add_permits(1);
    }
}
