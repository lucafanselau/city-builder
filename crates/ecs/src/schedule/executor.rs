use crate::{resource::Resources, schedule::scheduler::Scheduler};
use hecs::World;

/// Types that can execute a Scheduler's schedule
pub trait ScheduleExecutor {
    fn execute(schedule: &mut Scheduler, world: &mut World, resources: &mut Resources);
}

/// Really really basic sequential executor (should be replaced by a multi-threaded one
/// but atm we dont really need a more sophisticated one (and i want to grow more comfortable with
/// Rusts multi-threading and async features
#[allow(dead_code)]
pub struct SequentialExecutor;

impl ScheduleExecutor for SequentialExecutor {
    fn execute(schedule: &mut Scheduler, world: &mut World, resources: &mut Resources) {
        // #[derive(Debug)]
        // struct StageInfo {
        //     name: String,
        //     elapsed: f32,
        //     percentage: f32,
        // }
        // let mut profile_data = Vec::new();
        // let mut time = Instant::now();
        // let mut add_data = |a: &str| {
        //     profile_data.push(StageInfo {
        //         name: String::from(a),
        //         elapsed: time.elapsed().as_secs_f32(),
        //         percentage: 0.0,
        //     });
        //     time = Instant::now();
        // };

        for stage in schedule.order.iter() {
            // for now we will just execute each stage sequentially on one thread
            if let Some(systems) = schedule.stages.get(stage) {
                for system in systems.iter() {
                    // And then execute it
                    system.run(world, resources);
                }
            }
            //  add_data(stage);
        }
        // at the end we will execute the thread local ones
        for system in schedule.mut_systems.iter_mut() {
            system.run(world, resources)
        }
        // add_data("MUT_SYSTEMS");

        // {
        //     // Figure out percentage
        //     let total = profile_data.iter().fold(0.0f32, |acc, c| acc + c.elapsed);
        //     profile_data
        //         .iter_mut()
        //         .for_each(|d| d.percentage = (d.elapsed * 100f32) / total);
        // }

        // log::debug!("FRAME PROFILE");
        // profile_data.iter().for_each(|d| log::debug!("{:?}", d));
        // log::debug!("END PROFILE");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::into_system::IntoFunctionSystem;
    use std::cell::RefMut;

    fn first_system() {}
    fn second_system(mut counter: RefMut<i32>) {
        *counter += 1;
    }
    fn third_system(mut counter: RefMut<i32>) {
        *counter *= 5;
    }
    fn fourth_system(mut counter: RefMut<i32>) {
        *counter += 3;
    }
    fn fifth_system(mut counter: RefMut<i32>) {
        *counter /= 4;
    }

    #[test]
    fn simple_schedule() {
        let world = World::new();
        let resources = {
            let mut r = Resources::new();
            r.insert(0i32).unwrap();
            r
        };

        let mut scheduler = Scheduler::new();

        scheduler.add_stage("FIRST");
        scheduler.add_stage("SECOND");
        scheduler.add_stage("THIRD");
        scheduler.add_stage("FOURTH");
        scheduler.add_stage("FIFTH");

        // And add the systems
        scheduler.add_system_to_stage("FIRST", first_system.into_system());
        scheduler.add_system_to_stage("SECOND", second_system.into_system());
        scheduler.add_system_to_stage("THIRD", third_system.into_system());
        scheduler.add_system_to_stage("FOURTH", fourth_system.into_system());
        scheduler.add_system_to_stage("FIFTH", fifth_system.into_system());

        SequentialExecutor::execute(&scheduler, &world, &resources);

        // and test
        assert_eq!(*resources.get::<i32>().unwrap(), 2);
    }
}
