use super::astar::AStarScheduler;
use super::{BackwardsMachine, ScheduleInfo};

pub struct Dijkstra;

impl AStarScheduler for Dijkstra {
    fn estimate_remaining_cost(
        &mut self,
        _info: ScheduleInfo,
        _state: &BackwardsMachine,
        _cost: u32,
    ) -> u32 {
        0
    }
}

#[derive(Debug, Clone, Default)]
pub struct Guessooor {
    blocked_to_distance: f32,
}

impl Guessooor {
    pub fn new(blocked_to_distance: f32) -> Self {
        Self {
            blocked_to_distance,
            ..Default::default()
        }
    }
}

impl AStarScheduler for Guessooor {
    fn estimate_remaining_cost(
        &mut self,
        _info: ScheduleInfo,
        state: &BackwardsMachine,
        _cost: u32,
    ) -> u32 {
        let total_blocked = state.blocked_by.iter().map(|b| b.unwrap_or(0)).sum::<u32>();
        (total_blocked as f32 * self.blocked_to_distance).round() as u32
    }

    fn estimate_explored_map_size(
        &mut self,
        info: ScheduleInfo,
        _start_state: &BackwardsMachine,
    ) -> usize {
        let total_nodes = info.nodes.len();
        total_nodes * total_nodes * 300
    }
}
