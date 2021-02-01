pub mod camera;
pub mod components;
pub mod factory;
pub mod mesh;
mod renderer;

use app::*;

pub fn init_artisan(app: &mut App) {
    // First add a camera
    camera::init(app);
    // And now we will add the render system
    renderer::init(app);
}
