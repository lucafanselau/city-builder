pub mod components;
mod renderer;

use app::*;

pub fn init_artisan(app: &mut App) {
    // And now we will add the render system
    renderer::init(app);
}
