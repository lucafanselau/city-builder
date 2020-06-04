use imgui::{Context, ImString};
use imgui_winit_support::{HiDpiMode, WinitPlatform};

use winit::{event::Event, window::Window};

use std::{rc::Rc, sync::Arc, time::Instant};

use gfx_hal::Backend;

pub struct ImGuiRenderer<B: Backend>
where
    B::Device: Send + Sync,
{
    platform: WinitPlatform,
    imgui: Context,
    last_frame: Instant,
    window: Rc<Window>,
    device: Arc<B::Device>,
}

// Because Ui is not Debug
use std::fmt;
impl<B: Backend> fmt::Debug for ImGuiRenderer<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImGuiRenderer")
            .field("platform", &self.platform)
            .field("imgui", &self.imgui)
            .field("last_frame", &self.last_frame)
            .field("window", &self.window)
            .field("device", &self.device)
            .finish()
    }
}

impl<B: Backend> ImGuiRenderer<B> {
    pub fn new(
        window: Rc<Window>,
        device: Arc<B::Device>,
    ) -> Result<ImGuiRenderer<B>, Box<dyn std::error::Error>> {
        let mut imgui = Context::create();
        // Configuration

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);

        imgui.set_renderer_name(Some(ImString::new("citybuilder-imgui-renderer")));

        // Renderer Setup
        // We need a Pipeline, Buffers, Font Texture and


        Ok(ImGuiRenderer {
            platform,
            imgui,
            last_frame: Instant::now(),
            window,
            device,
        })
    }

    // pub fn get_frame(&'lf self) -> Option<&Ui<'lf>> {
    //     // self.frame.as_ref()
    // }

    pub fn handle_event<T>(&mut self, event: &Event<T>) {
        use std::borrow::Borrow;
        match event {
            Event::NewEvents(_) => {
                self.last_frame = self.imgui.io_mut().update_delta_time(self.last_frame);
            }
            Event::MainEventsCleared => (),
            // self.platform
            //     .prepare_frame(self.imgui.as_mut().expect("").borrow_mut().io_mut(), self.window.borrow())
            //     .expect("failed to prepare frame");
            // let mut context = self.imgui.take().expect("");
            // self.frame = Some(OwningHandle::new_with_fn(context, |ctx| {
            // 		unsafe {
            // 				let cell: &RefCell<Context> = ctx.as_ref().expect("Pointer was null");
            // 		// let mut c: &mut Context = cell.borrow_mut();
            // 				Box::new(cell.borrow_mut().frame())
            // 		}
            // }));
            Event::RedrawRequested(_) => (),
            // self.platform.prepare_render(
            //     &self.frame.expect(""),
            //     self.window.borrow(),
            // );
            // let frame = self.frame.take().expect("");
            // let draw_data = frame.render();
            // self.imgui = Some(frame.into_owner());

            // TODO: Render that bitch
            event => self
                .platform
                .handle_event(self.imgui.io_mut(), self.window.borrow(), &event),
        }
    }
}
