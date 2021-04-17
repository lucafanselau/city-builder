use crate::{mesh::Vertex, UP};

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
    cube(glam::vec3(1f32, 1f32, 1f32))
}

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

pub fn cube_at(scale: glam::Vec3, at: glam::Vec3) -> Vec<Vertex> {
    let matrix = glam::Mat4::from_translation(at);
    cube(scale)
        .into_iter()
        .map(|v| Vertex {
            pos: matrix.transform_point3(v.pos),
            normal: v.normal,
        })
        .collect()
}

/// Function to calculate a beam between from and to
/// width defaults to 0.1 if equal to None
pub fn beam(from: glam::Vec3, to: glam::Vec3, width: Option<f32>) -> Vec<Vertex> {
    let width = width.unwrap_or(0.1);
    let dir = to - from;
    let vertices = cube(glam::vec3(width, dir.length(), width));
    // now we have the vertices of an upward standing box that already has the correct length and centered at (0, 0, 0)
    // we will need to apply two transformations, first we will rotate the beam and then translate the vertices to the mid point between from and to
    let rotation_axis = UP.cross(dir).normalize();
    let angle = dir.angle_between(UP);
    let matrix = glam::Mat4::from_rotation_translation(
        glam::Quat::from_axis_angle(rotation_axis, angle),
        from + 0.5 * dir,
    );
    let normal_matrix: glam::Mat3 = matrix.inverse().transpose().into();
    vertices
        .into_iter()
        .map(|v| Vertex {
            pos: matrix.transform_point3(v.pos),
            normal: normal_matrix * v.normal,
        })
        .collect()
}
