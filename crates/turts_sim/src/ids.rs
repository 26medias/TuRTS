use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdAllocators {
    pub(crate) next_unit: u32,
    pub(crate) next_node: u32,
    pub(crate) next_job: u64,
}
impl Default for IdAllocators {
    fn default() -> Self {
        Self {
            next_unit: 1,
            next_node: 1,
            next_job: 1,
        }
    }
}
impl IdAllocators {
    pub fn next_values(&self) -> (u32, u32, u64) {
        (self.next_unit, self.next_node, self.next_job)
    }
    pub fn set_next_values(&mut self, unit: u32, node: u32, job: u64) {
        self.next_unit = unit;
        self.next_node = node;
        self.next_job = job;
    }
    pub(crate) fn allocate_unit(&mut self) -> Result<u32, String> {
        let id = self.next_unit;
        self.next_unit = id
            .checked_add(1)
            .ok_or_else(|| "unit id exhausted".to_string())?;
        Ok(id)
    }
    pub(crate) fn allocate_node(&mut self) -> Result<NodeId, String> {
        let id = self.next_node;
        self.next_node = id
            .checked_add(1)
            .ok_or_else(|| "node id exhausted".to_string())?;
        Ok(NodeId(id))
    }
    #[allow(dead_code)]
    pub(crate) fn allocate_job(&mut self) -> Result<JobId, String> {
        let id = self.next_job;
        self.next_job = id
            .checked_add(1)
            .ok_or_else(|| "job id exhausted".to_string())?;
        Ok(JobId(id))
    }
}
