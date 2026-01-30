pub mod hwaccel;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::pipeline::hwaccel::HardwareDecoder;
use crate::codec::simd::yuv_convert_avx512;
use crate::v4l2::device::Device;

pub struct Pipeline {
    decoder: Mutex<HardwareDecoder>,
    output_device: Device,
}

unsafe impl Send for Pipeline {}
unsafe impl Sync for Pipeline {}

impl Pipeline {
    pub fn new(video_nr: u16) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let decoder = HardwareDecoder::new()?;
        let output_device = Device::open(video_nr)?;
        
        Ok(Arc::new(Self {
            decoder: Mutex::new(decoder),
            output_device,
        }))
    }

    pub async fn process_chunk(&self, data: &[u8], width: usize, height: usize) -> Result<(), Box<dyn std::error::Error>> {
        let mut decoder = self.decoder.lock().await;
        
        // 1. Décodage matériel H264 -> YUV420
        let frame_ptr = decoder.decode(data)?;
        let frame = crate::pipeline::hwaccel::FrameWrapper(frame_ptr);
        
        unsafe {
            let av_frame = &*frame;
            
            // 2. Conversion SIMD YUV420 -> YUYV (format attendu par V4L2 loopback)
            // On prépare un buffer pour le YUYV (width * height * 2 octets)
            let mut yuyv_buffer = vec![0u8; width * height * 2];
            
            let y_plane = std::slice::from_raw_parts((*frame.0).data[0], ((*frame.0).linesize[0] as usize) * height);
            let u_plane = std::slice::from_raw_parts((*frame.0).data[1], ((*frame.0).linesize[1] as usize) * height / 2);
            let v_plane = std::slice::from_raw_parts((*frame.0).data[2], ((*frame.0).linesize[2] as usize) * height / 2);
            
            yuv_convert_avx512::yuv420_to_yuyv_avx512(
                y_plane,
                u_plane,
                v_plane,
                &mut yuyv_buffer,
                width,
                height
            );
            
            // 3. Envoi vers V4L2
            self.output_device.write_frame_dmabuf(&yuyv_buffer)?;
        }
        
        Ok(())
    }
}
