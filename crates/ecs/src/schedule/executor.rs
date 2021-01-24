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
        for stage in schedule.order.iter() {
            // for now we will just execute each stage sequentially on one thread
            if let Some(systems) = schedule.stages.get(stage) {
                for system in systems.iter() {
                    // And then execute it
                    system.run(world, resources);
                }
            }
        }
        // at the end we will execute the thread local ones
        for system in schedule.mut_systems.iter_mut() {
            system.run(world, resources)
        }
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
