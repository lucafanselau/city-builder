// General definitions
mod def;
pub use def::*;

pub mod camera;
pub mod components;
pub mod factory;
pub mod material;
pub mod mesh;
mod renderer;

use app::*;

pub mod prelude {
    pub use glam;
}

pub fn init_artisan(app: &mut App) {
    // First add a camera
    camera::init(app);
    // And now we will add the render system
    renderer::init(app);
}
