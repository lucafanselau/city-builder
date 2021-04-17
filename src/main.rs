use std::{any::TypeId, cell::Ref};

use app::{AssetHandle, AssetServer, IntoFunctionSystem, QueryBorrow, Timing};
use artisan::{components::Transform, mesh::Model, prelude::glam};

mod logger;
mod world;

fn spawner_plugin(app: &mut app::App) {
    let model_id: AssetHandle<Model> = {
        let server = app.get_res::<AssetServer>();
        // Create a sample entity
        server.load_asset("assets/meshes/GartenFarbig.gltf")
    };

    // let _entity = app.get_world_mut().spawn((
    //     artisan::components::ModelComponent(model_id),
    //     artisan::components::Transform::UNIT,
    // ));

    // fn movement_system(timing: Ref<Timing>, mut query: QueryBorrow<&mut Transform>) {
    //     let scale = ((timing.total_elapsed().sin() * 0.05) + 1.0) * glam::Vec3::one();
    //     for (_e, t) in query.iter() {
    //         t.set_scale(scale);
    //     }
    // }
    // app.add_system(app::stages::UPDATE, movement_system.into_system());
    world::spawn_world(app);
}

fn main() {
    logger::init_logger();
    let mut app = app::App::new();
    app.add_plugin(window::init_window);
    app.add_plugin(artisan::init_artisan);
    app.add_plugin(models::init_models);
    app.add_plugin(spawner_plugin);

    app.run();
}
