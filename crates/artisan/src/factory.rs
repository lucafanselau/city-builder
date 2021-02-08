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
    cube(glam::vec3(1f32, 1f32, 1f32))
}

pub fn cube(scale: glam::Vec3) -> Vec<Vertex> {
    let x = scale.x / 2f32;
    let y = scale.y / 2f32;
    let z = scale.z / 2f32;
    vec![
        Vertex {
            pos: glam::vec3(-x, -y, -z),
            normal: glam::vec3(-1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, z),
            normal: glam::vec3(-1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, z),
            normal: glam::vec3(-1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, -z),
            normal: glam::vec3(0.0, 0.0, -1.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, -z),
            normal: glam::vec3(0.0, 0.0, -1.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, -z),
            normal: glam::vec3(0.0, 0.0, -1.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, z),
            normal: glam::vec3(0.0, -1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, -z),
            normal: glam::vec3(0.0, -1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, -z),
            normal: glam::vec3(0.0, -1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, -z),
            normal: glam::vec3(0.0, 0.0, -1.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, -z),
            normal: glam::vec3(0.0, 0.0, -1.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, -z),
            normal: glam::vec3(0.0, 0.0, -1.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, -z),
            normal: glam::vec3(-1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, z),
            normal: glam::vec3(-1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, -z),
            normal: glam::vec3(-1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, z),
            normal: glam::vec3(0.0, -1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, z),
            normal: glam::vec3(0.0, -1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, -z),
            normal: glam::vec3(0.0, -1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, z),
            normal: glam::vec3(0.0, 0.0, 1.0),
        },
        Vertex {
            pos: glam::vec3(-x, -y, z),
            normal: glam::vec3(0.0, 0.0, 1.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, z),
            normal: glam::vec3(0.0, 0.0, 1.0),
        },
        Vertex {
            pos: glam::vec3(x, y, z),
            normal: glam::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, -z),
            normal: glam::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, -z),
            normal: glam::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, -z),
            normal: glam::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, z),
            normal: glam::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, z),
            normal: glam::vec3(1.0, 0.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, z),
            normal: glam::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, -z),
            normal: glam::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, -z),
            normal: glam::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, z),
            normal: glam::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, -z),
            normal: glam::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, z),
            normal: glam::vec3(0.0, 1.0, 0.0),
        },
        Vertex {
            pos: glam::vec3(x, y, z),
            normal: glam::vec3(0.0, 0.0, 1.0),
        },
        Vertex {
            pos: glam::vec3(-x, y, z),
            normal: glam::vec3(0.0, 0.0, 1.0),
        },
        Vertex {
            pos: glam::vec3(x, -y, z),
            normal: glam::vec3(0.0, 0.0, 1.0),
        },
    ]
}
