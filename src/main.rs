use std::any::TypeId;

mod logger;

fn spawner_plugin(app: &mut app::App) {
    log::info!("{:?}", TypeId::of::<artisan::mesh::MeshMap>());
    // Create a sample entity
    let mesh_id = {
        let mut mesh_map = app
            .get_resources()
            .get_mut::<artisan::mesh::MeshMap>()
            .expect("[main] failed to get mesh_map");
        mesh_map.load_mesh("Simple Circle", artisan::factory::circle(0.33, 360))
    };

    let _entity = app
        .get_world_mut()
        .spawn((artisan::components::MeshComponent(mesh_id),));
}

fn main() {
    logger::init_logger();
    let mut app = app::App::new();
    app.add_plugin(window::init_window);
    app.add_plugin(artisan::init_artisan);
    app.add_plugin(spawner_plugin);

    app.run();
}
