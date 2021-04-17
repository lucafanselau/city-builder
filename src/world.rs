use app::{stages, AssetHandle, AssetServer, Events, IntoFunctionSystem, Res};
use artisan::{
    camera::Camera,
    components::{ModelComponent, Transform},
    material::Color,
    mesh::{Mesh, MeshPart, Model, Vertex},
    prelude::glam::{self, IVec2, Vec3},
    renderer::ActiveContext,
    UP,
};
use noise::{MultiFractal, NoiseFn};
use std::{borrow::Borrow, collections::HashMap, sync::Arc};
use tasks::futures::future;
use window::{events::CursorMoved, WindowState};

use artisan::material::Material;

#[derive(Debug)]
pub struct World {
    // model: AssetHandle<
    debug_mesh: AssetHandle<Mesh>,
    height_map: HashMap<IVec2, f32>,
}

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

struct Plane {
    normal: glam::Vec3,
    base_point: glam::Vec3,
}

#[derive(Copy, Clone)]
struct Ray {
    origin: glam::Vec3,
    dir: glam::Vec3,
}

fn ray_plane_intersection(p: &Plane, r: &Ray) -> Option<glam::Vec3> {
    let dir_normal_dot = r.dir.dot(p.normal);
    if dir_normal_dot == 0.0 {
        None
    } else {
        // let delta = (p.base_point.dot(p.normal) - r.origin.dot(p.normal)) / dir_normal_dot;
        let delta = (p.base_point - r.origin).dot(p.normal) / dir_normal_dot;
        if delta < 0.0 {
            None
        } else {
            Some(r.origin + r.dir * delta)
        }
    }
}

fn test_triangle(r: Ray, a: glam::Vec3, b: glam::Vec3, c: glam::Vec3) -> Option<Vec3> {
    let plane = Plane {
        normal: (c - a).cross(b - a).normalize(),
        base_point: a,
    };

    ray_plane_intersection(&plane, &r)
        .map(|p| {
            // This method is based on barycentric coordinates as described in this answer
            // https://math.stackexchange.com/questions/4322/check-whether-a-point-is-within-a-3d-triangle
            let area = (b - a).cross(c - a).length() / 2.0;
            let alpha = (b - p).cross(c - p).length() / (2.0 * area);
            let beta = (c - p).cross(a - p).length() / (2.0 * area);
            let gamma = 1.0 - alpha - beta;
            // Now the point is in the triangle if and only if alpha, beta and gamma are in rangle 0..1
            let range = 0.0..=1.0;
            if range.contains(&alpha) && range.contains(&beta) && range.contains(&gamma) {
                Some(p)
            } else {
                None
            }
        })
        .flatten()
}

const PICKING_RANGE: i32 = 52;

fn mouse_picking(
    camera: Res<Camera>,
    window: Res<WindowState>,
    cursor_moved: Res<Events<CursorMoved>>,
    context: Res<Arc<ActiveContext>>,
    asset_server: Res<AssetServer>,
    world: Res<World>,
) {
    if let Some(CursorMoved { relative, .. }) = cursor_moved.iter().last() {
        let ray = camera.mouse_ray(
            relative,
            window.size.width as f32 / window.size.height as f32,
        );

        let pos = camera.eye;

        let ray = Ray {
            origin: pos,
            dir: ray,
        };

        let height = |at: &glam::IVec2| -> Option<f32> { world.height_map.get(at).copied() };

        let test_tile = |x: i32, z: i32| -> Option<glam::Vec3> {
            let pos = glam::ivec2(x, z);
            let base = glam::vec3(x as _, 0.0, z as _);

            let v00 = base + glam::vec3(0.0, height(&pos)?, 0.0);
            let v01 = base + glam::vec3(0.0, height(&(pos + glam::ivec2(0, 1)))?, 1.0);
            let v10 = base + glam::vec3(1.0, height(&(pos + glam::ivec2(1, 0)))?, 0.0);
            let v11 = base + glam::vec3(1.0, height(&(pos + glam::ivec2(1, 1)))?, 1.0);

            test_triangle(ray, v00, v01, v10).or_else(|| test_triangle(ray, v11, v01, v10))
        };

        let position = pos.floor().as_i32();
        let base_x = position.x;
        let base_z = position.z;

        let mut intersection = None;
        'outer: for x in 0..PICKING_RANGE {
            for z in 0..PICKING_RANGE {
                if let Some(i) = test_tile(base_x + x, base_z + z) {
                    intersection = Some(i);
                    break 'outer;
                }

                if x > 0 {
                    if let Some(i) = test_tile(base_x - x, base_z + z) {
                        intersection = Some(i);
                        break 'outer;
                    }
                }

                if z > 0 {
                    if let Some(i) = test_tile(base_x + x, base_z - z) {
                        intersection = Some(i);
                        break 'outer;
                    }
                }

                if x > 0 && z > 0 {
                    if let Some(i) = test_tile(base_x - x, base_z - z) {
                        intersection = Some(i);
                        break 'outer;
                    }
                }
            }
        }
        if let Some(intersection) = intersection {
            let mut parts = Vec::new();
            let mut vertices = debug_rect(
                HeightOption::Terrain(&world.height_map),
                intersection.x.floor() as _,
                intersection.z.floor() as _,
            );
            vertices.extend(artisan::factory::cube_at(
                glam::Vec3::splat(0.2),
                intersection,
            ));
            parts.push(MeshPart::from_data(
                "first_debug_mesh_part",
                &vertices,
                Material::BRONZE,
                &context,
            ));
            if !parts.is_empty() {
                // log::info!("NEW PARTS");
                let mesh = Mesh::new("debug_mesh", parts);
                future::block_on(asset_server.update_asset(&world.debug_mesh, mesh));
            }
        }
    }
}

enum HeightOption<'a> {
    Terrain(&'a HashMap<IVec2, f32>),
    Fixed(f32),
}

fn debug_rect(height_option: HeightOption, x: i32, z: i32) -> Vec<Vertex> {
    let vertex = |o_x: i32, o_z: i32| -> Vertex {
        let height = match height_option {
            HeightOption::Terrain(hm) => {
                hm.get(&glam::ivec2(x + o_x, z + o_z))
                    .expect("failed to get height")
                    + 0.01f32
            }
            HeightOption::Fixed(h) => h,
        };

        Vertex {
            pos: glam::vec3((x + o_x) as f32, height, (z + o_z) as f32),
            normal: UP,
        }
    };

    let v00 = vertex(0, 0);
    let v10 = vertex(1, 0);
    let v01 = vertex(0, 1);
    let v11 = vertex(1, 1);

    vec![v00, v10, v01, v01, v10, v11]
}

pub fn spawn_world(app: &mut app::App) {
    app.add_system(stages::UPDATE, mouse_picking.into_system());

    let mut vertices = HashMap::new();
    vertices.insert(GroundType::Steep, Vec::new());
    vertices.insert(GroundType::Mild, Vec::new());
    vertices.insert(GroundType::Snow, Vec::new());

    let noise_generator = noise::Fbm::new().set_octaves(14);
    let mut height_map = HashMap::new();

    let mut pos = |x: i32, z: i32| -> glam::Vec3 {
        let height = noise_generator.get([x as f64 / 256.0, z as f64 / 256.0]) as f32 * 60.0;
        height_map.insert(glam::ivec2(x, z), height);
        glam::vec3(x as _, height, z as _)
    };

    const WORLD_WIDTH: i32 = 32;

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
        model.add_mesh(glam::Mat4::IDENTITY, mesh);

        asset_server.add_loaded_asset("world-model", model)
    };

    let (debug_mesh, debug_model) = {
        let asset_server = app.get_res::<AssetServer>();
        let context = app.get_res::<Arc<ActiveContext>>();
        let vertices = debug_rect(HeightOption::Terrain(&height_map), 0, 0);
        let part = MeshPart::from_data(
            "debug_mesh_part",
            vertices.as_slice(),
            Material::BRONZE,
            &context,
        );
        let mesh = Mesh::new("debug_mesh", vec![part]);
        let mesh = asset_server.add_loaded_asset("debug_mesh", mesh);

        let mut model = Model::new();
        model.add_mesh(glam::Mat4::IDENTITY, mesh.clone_strong().unwrap());

        let model = asset_server.add_loaded_asset("debug_mesh_model", model);

        (mesh, model)
    };

    let world = World {
        height_map,
        debug_mesh,
    };
    app.insert_resource(world);

    app.get_world_mut()
        .spawn((ModelComponent(model), Transform::UNIT));

    app.get_world_mut()
        .spawn((ModelComponent(debug_model), Transform::UNIT));
}
