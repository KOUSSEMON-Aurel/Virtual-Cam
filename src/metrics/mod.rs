use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
pub struct ServerMetrics {
    pub packet_count: AtomicU64,
    pub bytes_received: AtomicU64,
    pub width: AtomicU64,
    pub height: AtomicU64,
}

impl ServerMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            packet_count: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            width: AtomicU64::new(1280),
            height: AtomicU64::new(720),
        })
    }

    pub fn record_packet(&self, bytes: u64) {
        self.packet_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn update_resolution(&self, w: u64, h: u64) {
        self.width.store(w, Ordering::Relaxed);
        self.height.store(h, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            packet_count: self.packet_count.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            width: self.width.load(Ordering::Relaxed),
            height: self.height.load(Ordering::Relaxed),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct MetricsSnapshot {
    pub packet_count: u64,
    pub bytes_received: u64,
    pub width: u64,
    pub height: u64,
}
