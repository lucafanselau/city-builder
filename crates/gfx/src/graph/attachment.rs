use generational_arena::Index;

#[derive(Debug, Clone)]
pub enum AttachmentIndex {
    Backbuffer,
    Custom(Index),
}
