mod renderer;

use std::sync::Arc;

use app::*;
use gfx::gfx_context::Context as GfxContext;
use render::prelude::*;

#[derive(Debug)]
pub struct RawRenderContext<Context: GpuContext> {
    pub ctx: Arc<Context>,
    pub resources: GpuResources<Context>,
}

impl<Context: GpuContext> Drop for RawRenderContext<Context> {
    fn drop(&mut self) {
        self.ctx.wait_idle();
    }
}

pub type RenderContext = RawRenderContext<GfxContext>;

pub fn init_artisan(app: &mut App) {
    {
        let resources = app.get_resources();
        let context = {
            let window_state = resources
                .get::<window::WindowState>()
                .expect("[Artisan] failed to load window");

            let ctx = Arc::new(GfxContext::new(&window_state.window));
            RenderContext {
                ctx: ctx.clone(),
                resources: GpuResources::<GfxContext>::new(ctx),
            }
        };

        resources
            .insert(context)
            .expect("[Artisan] failed to insert render context");
    }

    // And now we will add the render system
    renderer::init(app);
}
