use std::{collections::HashMap, sync::Arc};

use app::AssetServer;
use artisan::{
    material::Color,
    mesh::{Mesh, MeshPart, Model, Vertex},
    prelude::glam,
    renderer::ActiveContext,
    UP,
};
use noise::{MultiFractal, NoiseFn};

#[derive(Debug, Hash, PartialEq, Eq)]
enum GroundType {
    Steep,
    Mild,
    Snow,
}

fn get_type(height: f32, angle: f32) -> GroundType {
    if angle > 25.0f32.to_radians() {
        GroundType::Steep
    } else if height > 5.0 {
        GroundType::Snow
    } else {
        GroundType::Mild
    }
}
fn to_color(ground_type: &GroundType) -> Color {
    match ground_type {
        GroundType::Steep => Color::from_rgba(48, 54, 51, 1.0),
        GroundType::Snow => Color::from_rgba(235, 242, 250, 1.0),
        GroundType::Mild => Color::from_rgba(112, 174, 110, 1.0),
    }
}

pub fn spawn_world(app: &mut app::App) {
    let mut vertices = HashMap::new();
    vertices.insert(GroundType::Steep, Vec::new());
    vertices.insert(GroundType::Mild, Vec::new());
    vertices.insert(GroundType::Snow, Vec::new());

    let noise_generator = noise::Fbm::new().set_octaves(14);

    let pos = |x: i32, z: i32| -> glam::Vec3 {
        glam::vec3(
            x as _,
            noise_generator.get([x as f64 / 256.0, z as f64 / 256.0]) as f32 * 60.0,
            z as _,
        )
    };

    const WORLD_WIDTH: i32 = 127;

    for x in -WORLD_WIDTH..=WORLD_WIDTH {
        for z in -WORLD_WIDTH..=WORLD_WIDTH {
            let v00 = pos(x, z);
            let v10 = pos(x + 1, z);
            let v01 = pos(x, z + 1);
            let v11 = pos(x + 1, z + 1);

            let height = [v00.y, v01.y, v10.y, v11.y].iter().sum::<f32>() / 4.0;

            let normal0 = (v01 - v00).cross(v10 - v00);
            let normal1 = (v10 - v11).cross(v01 - v11);

            let ground_type0 = get_type(height, UP.angle_between(normal0));
            let ground_type1 = get_type(height, UP.angle_between(normal1));

            {
                let vertices = vertices.entry(ground_type0).or_insert_with(Vec::new);
                vertices.push(Vertex {
                    pos: v00,
                    normal: normal0,
                });
                vertices.push(Vertex {
                    pos: v10,
                    normal: normal0,
                });
                vertices.push(Vertex {
                    pos: v01,
                    normal: normal0,
                });
            }
            {
                let vertices = vertices.entry(ground_type1).or_insert_with(Vec::new);
                vertices.push(Vertex {
                    pos: v01,
                    normal: normal1,
                });
                vertices.push(Vertex {
                    pos: v10,
                    normal: normal1,
                });
                vertices.push(Vertex {
                    pos: v11,
                    normal: normal1,
                });
            }
        }
    }

    let parts: Vec<_> = vertices
        .iter()
        .map(|(t, vertices)| {
            let context = app.get_res::<Arc<ActiveContext>>();
            MeshPart::from_data("world", &vertices, to_color(t).into(), &context)
        })
        .collect();

    let model = {
        let asset_server = app.get_res::<AssetServer>();

        let mesh = Mesh::new("world", parts);
        let mesh = asset_server.add_loaded_asset("world", mesh);

        let mut model = Model::new();
        model.add_mesh(glam::Mat4::identity(), mesh);

        asset_server.add_loaded_asset("world-model", model)
    };

    app.get_world_mut().spawn((
        artisan::components::ModelComponent(model),
        artisan::components::Transform::UNIT,
    ));
}
