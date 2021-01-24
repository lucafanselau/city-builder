use raw_window_handle::HasRawWindowHandle;

use crate::prelude::GpuContext;

pub trait GpuBuilder {
    type Context: GpuContext;

    fn new() -> Self;

    fn create_surface<W: HasRawWindowHandle>(
        &self,
        window: &W,
    ) -> <Self::Context as GpuContext>::SurfaceHandle;

    fn build(self) -> Self::Context;
}
