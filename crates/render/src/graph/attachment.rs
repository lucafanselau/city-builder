use crate::util::format::TextureFormat;

#[derive(Debug, Clone)]
pub enum AttachmentSize {
    Relative(f32, f32),
    Absolute(u32, u32),
}

#[derive(Debug)]
pub struct GraphAttachment {
    pub(crate) size: AttachmentSize,
    pub(crate) format: TextureFormat,
    pub(crate) is_backbuffer: bool,
}

impl GraphAttachment {
    pub fn new(size: AttachmentSize, format: TextureFormat) -> Self {
        Self {
            size,
            format,
            is_backbuffer: false,
        }
    }
}
