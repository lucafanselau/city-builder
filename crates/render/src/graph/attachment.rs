use std::borrow::Cow;

use uuid::Uuid;

use crate::util::format::{ImageTiling, TextureFormat};

#[derive(Debug, Clone)]
pub enum AttachmentSize {
    Relative(f32, f32),
    Absolute(u32, u32),
}

impl AttachmentSize {
    pub const SWAPCHAIN: Self = Self::Relative(1.0, 1.0);
}

#[derive(Debug)]
pub struct GraphAttachment {
    // Note definitly not a perfect solution but should work for now
    pub id: Uuid,
    pub name: Cow<'static, str>,
    pub size: AttachmentSize,
    pub format: TextureFormat,
    pub is_backbuffer: bool,
    pub tiling: ImageTiling,
}

impl GraphAttachment {}

impl GraphAttachment {
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        size: AttachmentSize,
        format: TextureFormat,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            size,
            format,
            is_backbuffer: false,
            tiling: ImageTiling::Optimal,
        }
    }
}
