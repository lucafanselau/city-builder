use downcast_rs::{impl_downcast, DowncastSync};

pub trait Asset: DowncastSync {}
impl<T: DowncastSync> Asset for T {}
impl_downcast!(sync Asset);
