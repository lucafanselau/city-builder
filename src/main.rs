use std::{any::TypeId, cell::Ref};

use app::{IntoFunctionSystem, QueryBorrow, Timing};
use artisan::{components::Transform, prelude::glam};

mod logger;

fn spawner_plugin(app: &mut app::App) {
    log::info!("{:?}", TypeId::of::<artisan::mesh::MeshMap>());
    // Create a sample entity
    let mesh_id = {
        let mut mesh_map = app
            .get_resources()
            .get_mut::<artisan::mesh::MeshMap>()
            .expect("[main] failed to get mesh_map");
        // mesh_map.load_mesh("Simple Circle", artisan::factory::circle(0.33, 360))
        mesh_map.load_mesh("unit_cube", artisan::factory::unit_cube())
    };

    let _entity = app.get_world_mut().spawn((
        artisan::components::MeshComponent(mesh_id),
        artisan::material::MaterialComponent::BRONZE,
        artisan::components::Transform::UNIT,
    ));

    fn movement_system(timing: Ref<Timing>, mut query: QueryBorrow<&mut Transform>) {
        let scale = ((timing.total_elapsed().sin() * 0.5) + 1.0) * glam::Vec3::one();
        for (_e, t) in query.iter() {
            t.set_scale(scale);
        }
    }
    app.add_system(app::stages::UPDATE, movement_system.into_system());
}

fn main() {
    logger::init_logger();
    let mut app = app::App::new();
    app.add_plugin(window::init_window);
    app.add_plugin(artisan::init_artisan);
    app.add_plugin(spawner_plugin);

    app.run();
}
