use crate::mesh::Vertex;

pub fn circle(r: f32, resolution: u32) -> Vec<Vertex> {
    let mut result = Vec::new();
    let step = (360f32 / resolution as f32).to_radians();

    let normal = glam::vec3(0.0, 0.0, 1.0);

    let origin = Vertex {
        pos: glam::vec3(0.0, 0.0, 0.0),
        normal,
    };

    let mut iter = (0..=resolution).map(|i| i as f32 * step).peekable();
    while let Some(angle) = iter.next() {
        if let Some(next_angle) = iter.peek() {
            result.push(origin);
            // Vertex at current angle
            result.push(Vertex {
                pos: glam::vec3(angle.cos() * r, angle.sin() * r, 0.0),
                normal,
            });
            // Vertex at next angle
            result.push(Vertex {
                pos: glam::vec3(next_angle.cos() * r, next_angle.sin() * r, 0.0),
                normal,
            });
        }
    }

    result
}

pub fn unit_cube() -> Vec<Vertex> {
    log::info!(
        "[1, 2, 3] * [17, 23, 5] = {}",
        glam::vec3(1.0, 2.0, 3.0) * glam::vec3(17.0, 23.0, 5.0)
    );
    cube(glam::vec3(1f32, 1f32, 1f32))
}

const UP: glam::Vec3 = glam::const_vec3!([0.0, 1.0, 0.0]);
pub fn cube(scale: glam::Vec3) -> Vec<Vertex> {
    let scale = 0.5 * scale;

    let mut vertices = Vec::with_capacity(36);

    let mut calc_from_norm = |norm: glam::Vec3, orthogonal: glam::Vec3| {
        let base = norm * scale;
        let orthogonal = orthogonal * scale;

        let right = (base.cross(orthogonal)).normalize() * scale;

        let vec = |a: f32, b: f32| -> Vertex {
            Vertex {
                pos: base + (a * right + b * orthogonal),
                normal: norm,
            }
        };

        // First triangle
        vertices.push(vec(-1.0, 1.0));
        vertices.push(vec(-1.0, -1.0));
        vertices.push(vec(1.0, -1.0));

        // second triangle
        vertices.push(vec(-1.0, 1.0));
        vertices.push(vec(1.0, -1.0));
        vertices.push(vec(1.0, 1.0));
    };

    // X Normals
    calc_from_norm(glam::vec3(1.0, 0.0, 0.0), UP);
    calc_from_norm(glam::vec3(-1.0, 0.0, 0.0), UP);
    // Z Normals
    calc_from_norm(glam::vec3(0.0, 0.0, 1.0), UP);
    calc_from_norm(glam::vec3(0.0, 0.0, -1.0), UP);
    // Y Normals
    calc_from_norm(glam::vec3(0.0, 1.0, 0.0), glam::vec3(1.0, 0.0, 0.0));
    calc_from_norm(glam::vec3(0.0, -1.0, 0.0), glam::vec3(1.0, 0.0, 0.0));

    vertices
}
