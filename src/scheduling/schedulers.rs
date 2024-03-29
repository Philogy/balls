use super::astar::AStarScheduler;
use super::{BackwardsMachine, ScheduleInfo};

pub struct Dijkstra;

impl AStarScheduler for Dijkstra {
    fn estimate_remaining_cost(
        &self,
        _info: ScheduleInfo,
        _state: &BackwardsMachine,
        _cost: u32,
    ) -> u32 {
        0
    }
}

#[derive(Debug, Clone, Default)]
pub struct Guessooor(f32);

impl Guessooor {
    pub fn new(blocked_to_distance: f32) -> Self {
        Self(blocked_to_distance)
    }
}

impl AStarScheduler for Guessooor {
    fn estimate_remaining_cost(
        &self,
        _info: ScheduleInfo,
        state: &BackwardsMachine,
        _cost: u32,
    ) -> u32 {
        (state.total_blocked() as f32 * self.0).round() as u32
    }
}
