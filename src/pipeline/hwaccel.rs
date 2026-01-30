use ffmpeg_next as ffmpeg;
use std::ptr;

pub struct HardwareDecoder {
    decoder_ctx: *mut ffmpeg::ffi::AVCodecContext,
    _hw_device_ctx: *mut ffmpeg::ffi::AVBufferRef,
}

unsafe impl Send for HardwareDecoder {}
unsafe impl Sync for HardwareDecoder {}

impl HardwareDecoder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            ffmpeg::init()?;
            
            let codec = ffmpeg::ffi::avcodec_find_decoder(
                ffmpeg::ffi::AVCodecID::AV_CODEC_ID_H264
            );
            
            if codec.is_null() {
                return Err("H264 decoder not found".into());
            }

            let decoder_ctx = ffmpeg::ffi::avcodec_alloc_context3(codec);
            
            // Setup VAAPI
            let hw_type = ffmpeg::ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI;
            let mut hw_device_ctx: *mut ffmpeg::ffi::AVBufferRef = ptr::null_mut();
            
            ffmpeg::ffi::av_hwdevice_ctx_create(
                &mut hw_device_ctx,
                hw_type,
                ptr::null(),
                ptr::null_mut(),
                0,
            );
            
            (*decoder_ctx).hw_device_ctx = ffmpeg::ffi::av_buffer_ref(hw_device_ctx);
            
            ffmpeg::ffi::avcodec_open2(decoder_ctx, codec, ptr::null_mut());
            
            Ok(Self { 
                decoder_ctx, 
                _hw_device_ctx: hw_device_ctx 
            })
        }
    }
    
    pub fn decode(&mut self, data: &[u8]) -> Result<*mut ffmpeg::ffi::AVFrame, Box<dyn std::error::Error>> {
        unsafe {
            let packet = ffmpeg::ffi::av_packet_alloc();
            (*packet).data = data.as_ptr() as *mut u8;
            (*packet).size = data.len() as i32;
            
            ffmpeg::ffi::avcodec_send_packet(self.decoder_ctx, packet);
            ffmpeg::ffi::av_packet_free(&mut (packet as *mut ffmpeg::ffi::AVPacket));
            
            let mut frame = ffmpeg::ffi::av_frame_alloc();
            let ret = ffmpeg::ffi::avcodec_receive_frame(self.decoder_ctx, frame);
            
            if ret == 0 {
                Ok(frame)
            } else {
                ffmpeg::ffi::av_frame_free(&mut frame);
                Err("Need more data or error".into())
            }
        }
    }
}

// Wrapper pour libérer la frame automatiquement
pub struct FrameWrapper(pub *mut ffmpeg::ffi::AVFrame);
impl Drop for FrameWrapper {
    fn drop(&mut self) {
        unsafe { ffmpeg::ffi::av_frame_free(&mut self.0); }
    }
}

impl std::ops::Deref for FrameWrapper {
    type Target = ffmpeg::ffi::AVFrame;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl Drop for HardwareDecoder {
    fn drop(&mut self) {
        unsafe {
            ffmpeg::ffi::avcodec_free_context(&mut self.decoder_ctx);
        }
    }
}
