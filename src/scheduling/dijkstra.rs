use super::actions::ActionIterator;
use super::astar::AStarScheduler;
use super::BackwardsMachine;

pub struct Dijkstra;

impl AStarScheduler<ActionIterator> for Dijkstra {
    fn iter_actions(state: &BackwardsMachine) -> ActionIterator {
        ActionIterator::new(state)
    }
}
