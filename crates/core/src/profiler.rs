use std::time::Instant;

use cli_table::{format::Justify, print_stdout, Cell, Style, Table};

struct ProfilerStep {
    step_name: String,
    elapsed: f32,
    percentage: f32,
}

fn print_steps(name: &String, total: f32, data: &Vec<ProfilerStep>) {
    let table = data
        .iter()
        .map(|d| {
            vec![
                d.step_name.clone().cell(),
                format!("{:.2}", d.elapsed * 1000.0)
                    .cell()
                    .justify(Justify::Right),
                format!("{:.2}", d.percentage * 100.0)
                    .cell()
                    .justify(Justify::Right),
            ]
        })
        .table()
        .title(vec![
            "Step Name".cell().bold(true),
            "Elapsed (in ms)".cell().bold(true),
            "%".cell().bold(true),
        ])
        .bold(true);

    log::info!("=======================================");
    log::info!("PROFILER DATA for {} (Total {:.2}s)", name, total);
    print_stdout(table).expect("[Profiler] failed to print");
    log::info!("=======================================");
}

pub struct Profiler {
    name: String,
    data: Vec<ProfilerStep>,
    last_step: Instant,
}

impl Profiler {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            last_step: Instant::now(),
        }
    }

    pub fn step(&mut self, stage_name: impl Into<String>) {
        self.data.push(ProfilerStep {
            step_name: stage_name.into(),
            elapsed: self.last_step.elapsed().as_secs_f32(),
            percentage: 0.0,
        });
        self.last_step = Instant::now();
    }

    pub fn finish(mut self) {
        // Calculate percentage
        let total_elapsed = self.data.iter().fold(0.0f32, |acc, x| acc + x.elapsed);
        self.data
            .iter_mut()
            .for_each(|x| x.percentage = x.elapsed / total_elapsed);

        print_steps(&self.name, total_elapsed, &self.data);
    }
}
