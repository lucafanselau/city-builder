use std::borrow::Cow;

use crate::util::format::TextureFormat;

#[derive(Debug, Clone)]
pub enum AttachmentSize {
    Relative(f32, f32),
    Absolute(u32, u32),
}

#[derive(Debug)]
pub struct GraphAttachment {
    pub name: Cow<'static, str>,
    pub size: AttachmentSize,
    pub format: TextureFormat,
    pub is_backbuffer: bool,
}

impl GraphAttachment {
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        size: AttachmentSize,
        format: TextureFormat,
    ) -> Self {
        Self {
            name: name.into(),
            size,
            format,
            is_backbuffer: false,
        }
    }
}
