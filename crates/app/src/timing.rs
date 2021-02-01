use ecs::prelude::IntoFunctionSystem;

use crate::{stages, App};
use std::{cell::RefMut, time::Instant};

/// Default added timing resource for global usage
#[derive(Debug)]
pub struct Timing {
    startup: Instant,
    last_frame: Instant,
    pub dt: f32,
    // NOTE(luca): Will be removed later when we have profiling
    counter: f32,
    frames: u32,
}

impl Timing {
    fn new() -> Self {
        Self {
            startup: Instant::now(),
            last_frame: Instant::now(),
            dt: 0f32,
            counter: 0f32,
            frames: 0,
        }
    }

    pub fn total_elapsed(&self) -> f32 {
        self.startup.elapsed().as_secs_f32()
    }
}

fn timing_update(mut timing: RefMut<Timing>) {
    // Calculate delta time
    timing.dt = timing.last_frame.elapsed().as_secs_f32();
    // Update last frame
    timing.last_frame = Instant::now();

    // Calculate fps
    timing.counter += timing.dt;
    if timing.counter > 1f32 {
        timing.counter -= 1f32;
        log::info!("fps: {}", timing.frames);
        timing.frames = 0;
    }
    timing.frames += 1;
}

pub(crate) fn init(app: &mut App) {
    app.insert_resource(Timing::new());
    app.add_system(stages::PREPARE_FRAME, timing_update.into_system());
}
