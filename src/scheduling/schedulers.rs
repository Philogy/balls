use super::actions::ActionIterator;
use super::astar::AStarScheduler;
use super::BackwardsMachine;

pub struct Dijkstra;

impl AStarScheduler<ActionIterator> for Dijkstra {}

pub struct Guessooor(pub f32);

impl AStarScheduler<ActionIterator> for Guessooor {
    fn remaining_distance_heuristic(&mut self, state: &BackwardsMachine) -> u32 {
        let total_blocked = state.blocked_by.iter().map(|b| b.unwrap_or(0)).sum::<u32>();
        (total_blocked as f32 * self.0) as u32
    }
}
