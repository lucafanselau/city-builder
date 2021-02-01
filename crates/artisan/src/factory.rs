use crate::mesh::Vertex;

pub fn circle(r: f32, resolution: u32) -> Vec<Vertex> {
    let mut result = Vec::new();
    let step = (360f32 / resolution as f32).to_radians();

    let origin = Vertex {
        pos: glam::vec3(0.0, 0.0, 0.0),
    };

    let mut iter = (0..=resolution).map(|i| i as f32 * step).peekable();
    while let Some(angle) = iter.next() {
        if let Some(next_angle) = iter.peek() {
            result.push(origin);
            // Vertex at current angle
            result.push(Vertex {
                pos: glam::vec3(angle.cos() * r, angle.sin() * r, 0.0),
            });
            // Vertex at next angle
            result.push(Vertex {
                pos: glam::vec3(next_angle.cos() * r, next_angle.sin() * r, 0.0),
            });
        }
    }

    result
}
