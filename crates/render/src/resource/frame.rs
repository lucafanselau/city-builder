#[derive(Debug, Clone)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

#[derive(Debug, Clone)]
pub enum Clear {
    Color(f32, f32, f32, f32),
    Depth(f32, u32),
}
