pub mod device;
pub mod dmabuf;

pub fn setup_loopback(nr: u16) -> std::io::Result<()> {
    // Note: Nécessite v4l2loopback-dkms
    // sudo modprobe v4l2loopback video_nr=nr card_label="PhoneCam Ultimate"
    Ok(())
}
