use std::borrow::{Borrow, Cow};
use std::collections::HashMap;

use crate::system::{MutatingSystem, System};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InsertStageError {
    #[error("There is already a stage with this name")]
    StageExists,
    #[error("The anchor stage is missing")]
    AnchorStageMissing,
}

#[derive(Debug, Error)]
pub enum AddSystemError {
    #[error("The Stage specified is missing")]
    StageMissing,
}

pub struct Scheduler {
    pub(crate) stages: HashMap<Cow<'static, str>, Vec<Box<dyn System>>>,
    // NOTE(luca): Currently they will all be executed at the end
    pub(crate) mut_systems: Vec<Box<dyn MutatingSystem>>,
    pub(crate) order: Vec<Cow<'static, str>>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            stages: Default::default(),
            mut_systems: Default::default(),
            order: vec![],
        }
    }

    /// Just push back the stage in the ordering (eg. insert stage at the end of execution)
    pub fn try_add_stage(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<(), InsertStageError> {
        let name = name.into();
        if self.order.contains(name.borrow()) {
            Err(InsertStageError::StageExists)
        } else {
            self.order.push(name.clone());
            self.stages.insert(name, Default::default());
            Ok(())
        }
    }

    /// Will panic if stage already exists
    pub fn add_stage(&mut self, name: impl Into<Cow<'static, str>>) {
        self.try_add_stage(name).expect("stage already exists");
    }

    fn try_add_stage_relative(
        &mut self,
        anchor: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        offset: usize,
    ) -> Result<(), InsertStageError> {
        let name = name.into();
        let anchor = anchor.into();
        if self.order.contains(&name) {
            Err(InsertStageError::StageExists)
        } else if let Some(index) = self.order.iter().position(|s| s == &anchor) {
            self.order.insert(index + offset, name.clone());
            self.stages.insert(name, Default::default());
            Ok(())
        } else {
            Err(InsertStageError::AnchorStageMissing)
        }
    }

    pub fn try_add_stage_before(
        &mut self,
        anchor: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<(), InsertStageError> {
        self.try_add_stage_relative(anchor, name, 0)
    }

    pub fn add_stage_before(
        &mut self,
        anchor: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
    ) {
        self.try_add_stage_before(anchor, name)
            .expect("failed to insert stage before");
    }

    pub fn try_add_stage_after(
        &mut self,
        anchor: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
    ) -> Result<(), InsertStageError> {
        self.try_add_stage_relative(anchor, name, 1)
    }

    pub fn add_stage_after(
        &mut self,
        anchor: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
    ) {
        self.try_add_stage_after(anchor, name)
            .expect("failed to insert stage after");
    }

    pub fn try_add_system_to_stage(
        &mut self,
        stage: impl Into<Cow<'static, str>>,
        system: Box<dyn System>,
    ) -> Result<(), AddSystemError> {
        let name = stage.into();
        if let Some(systems) = self.stages.get_mut(&name) {
            systems.push(system);
            Ok(())
        } else {
            Err(AddSystemError::StageMissing)
        }
    }

    pub fn add_system_to_stage(
        &mut self,
        stage: impl Into<Cow<'static, str>>,
        system: Box<dyn System>,
    ) {
        self.try_add_system_to_stage(stage, system)
            .expect("failed to add system")
    }

    pub fn add_mut_system(&mut self, system: Box<dyn MutatingSystem>) {
        self.mut_systems.push(system)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::QueryBorrow;

    #[test]
    fn simple_schedule() {
        let mut scheduler = Scheduler::new();

        scheduler.add_stage("PRE_FRAME");
        scheduler.add_stage("UPDATE");
        scheduler.add_stage("RENDER");
        scheduler.add_stage("END_FRAME");

        // Before and after ordering
        scheduler.add_stage_before("UPDATE", "PRE_UPDATE");
        scheduler.add_stage_after("UPDATE", "POST_UPDATE");

        assert_eq!(
            scheduler.order,
            [
                "PRE_FRAME",
                "PRE_UPDATE",
                "UPDATE",
                "POST_UPDATE",
                "RENDER",
                "END_FRAME"
            ]
        );
    }

    #[test]
    #[should_panic(expected = "stage already exists")]
    fn same_stage() {
        let mut scheduler = Scheduler::new();
        scheduler.add_stage("UPDATE");
        // THIS SHOULD PANIC
        scheduler.add_stage("UPDATE");
    }

    fn a_system(_query: QueryBorrow<&i32>) {}

    use crate::system::into_system::IntoFunctionSystem;

    #[test]
    fn add_a_system() {
        let mut scheduler = Scheduler::new();
        scheduler.add_stage("TEST");
        scheduler.add_system_to_stage("TEST", a_system.into_system());
        assert_eq!(scheduler.order, ["TEST"]);
        assert!(scheduler.stages.contains_key("TEST"));
        assert_eq!(scheduler.stages.get("TEST").unwrap().len(), 1)
    }
}
