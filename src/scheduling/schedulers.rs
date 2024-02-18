use super::actions::ActionIterator;
use super::astar::{AStarScheduler, ExploredMap, ScheduleNode, ScheduleQueue};
use super::{BackwardsMachine, ScheduleInfo};

pub struct Dijkstra;

impl AStarScheduler<ActionIterator> for Dijkstra {
    type Summary = ();

    fn summarize(
        &mut self,
        _info: ScheduleInfo,
        _node: &super::astar::ScheduleNode,
        _steps: &Vec<super::Step>,
        _queue: &ScheduleQueue,
        _explored: &ExploredMap,
    ) -> Self::Summary {
    }
}

#[derive(Debug, Clone, Default)]
pub struct Guessooor {
    blocked_to_distance: f32,
    total_explored: usize,
    est_capacity: usize,
}

impl Guessooor {
    pub fn new(blocked_to_distance: f32) -> Self {
        Self {
            blocked_to_distance,
            ..Default::default()
        }
    }
}

impl AStarScheduler<ActionIterator> for Guessooor {
    type Summary = (u32, usize, f64);

    fn remaining_distance_heuristic(
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
        self.est_capacity = total_nodes * total_nodes * 300;
        self.est_capacity
    }

    fn on_explored_path(
        &mut self,
        _info: ScheduleInfo,
        _new_state: &BackwardsMachine,
        _new_cost: u32,
        _explored: &Option<&super::astar::Explored>,
    ) {
        self.total_explored += 1;
    }

    fn summarize(
        &mut self,
        _info: ScheduleInfo,
        node: &ScheduleNode,
        _steps: &Vec<super::Step>,
        _queue: &ScheduleQueue,
        explored: &ExploredMap,
    ) -> Self::Summary {
        (
            node.cost,
            self.total_explored,
            (self.est_capacity as f64 / explored.len() as f64),
        )
    }
}
