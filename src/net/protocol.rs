pub struct Header {
    pub magic: [u8; 2],
    pub frame_type: u8,
    pub flags: u8,
    pub payload_length: u32,
}

impl Header {
    pub const SIZE: usize = 8;
    pub const MAGIC: [u8; 2] = [0x50, 0x43]; // "PC"

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        if data[0] != Self::MAGIC[0] || data[1] != Self::MAGIC[1] {
            return None;
        }

        let payload_length = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        Some(Self {
            magic: [data[0], data[1]],
            frame_type: data[2],
            flags: data[3],
            payload_length,
        })
    }
}

pub enum FrameType {
    P = 0x00,
    I = 0x01,
    B = 0x02,
}
