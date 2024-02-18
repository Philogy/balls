use super::actions::{Action, ActionIterator};
use crate::scheduling::swap::Swapper;
use crate::scheduling::{BackwardsMachine, ScheduleInfo, Step};
use std::collections::{BinaryHeap, HashMap};
use xxhash_rust::xxh3::Xxh3Builder;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduleNode {
    pub state: BackwardsMachine,
    /// Real, known cost.
    pub cost: u32,
    /// Total cost (including heuristic).
    pub score: u32,
    pub at_end: bool,
}

impl Ord for ScheduleNode {
    /// Reverse compare (Greater means better i.e. score), makes use with
    /// `std::collections::BinaryHeap`'s max heap into a "min heap".
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Make sure lowest score lands up top.
        other
            .score
            .cmp(&self.score)
            // Prioritize done solutions if scores are identical
            .then(self.at_end.cmp(&other.at_end))
            // Otherwise choose node with longer actual distance (cost), makes sure we're
            // prioritizing solutions that are *closest* to the solution according to the
            // heuristic.
            .then(self.cost.cmp(&other.cost))
    }
}

impl PartialOrd for ScheduleNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
pub struct Explored {
    came_from: BackwardsMachine,
    steps: Vec<Step>,
    cost: u32,
}

pub type ExploredMap = HashMap<BackwardsMachine, Explored, Xxh3Builder>;
pub type ScheduleQueue = BinaryHeap<ScheduleNode>;

pub trait AStarScheduler<A>: Sized
where
    A: Iterator<Item = Action>,
{
    type Summary: Sized;

    fn schedule(
        mut self,
        info: ScheduleInfo,
        start: BackwardsMachine,
    ) -> (Vec<Step>, Self::Summary) {
        let mut queue: ScheduleQueue = BinaryHeap::new();
        let est_capacity = self.estimate_explored_map_size(info, &start);
        let mut explored: ExploredMap =
            HashMap::with_capacity_and_hasher(est_capacity, Xxh3Builder::default());
        self.on_schedule_start(info, &start);

        let score = self.remaining_distance_heuristic(info, &start, 0);
        queue.push(ScheduleNode {
            state: start.clone(),
            cost: 0,
            score,
            at_end: start.all_done(),
        });

        while let Some(node) = queue.pop() {
            if node.at_end {
                let mut state_key = &node.state;
                let mut all_steps = vec![];
                while let Some(e) = explored.get(state_key) {
                    all_steps.extend(e.steps.clone().into_iter().rev());
                    state_key = &e.came_from;
                }
                let summary = self.summarize(info, &node, &all_steps, &queue, &explored);
                return (all_steps, summary);
            }
            for action in ActionIterator::new(info, &node.state) {
                let mut new_state = node.state.clone();
                let mut steps = vec![];
                new_state.apply(info, action, &mut steps);
                let at_end = new_state.all_done();
                if at_end {
                    if new_state.stack.len() == 0 {
                        debug_assert_eq!(info.target_input_stack.len(), 0, "Ended with a stack of size 0 but target_input_stack has a non-zero length");
                    } else {
                        let mut swapper =
                            Swapper::new(&mut new_state.stack, info.target_input_stack);
                        match swapper.get_swaps() {
                            Ok(s) => steps.extend(s.into_iter().map(Step::Swap)),
                            Err(_) => continue,
                        }
                        assert!(
                            swapper.matching_count().unwrap(),
                            "Not-matching count according to swapper despite all_done => true"
                        );
                    }
                }
                let new_cost = node.cost + steps.iter().map(|step| step.cost()).sum::<u32>();
                let e = explored.get(&new_state);
                self.on_explored_path(info, &new_state, new_cost, &e);
                let new_cost_better = e.map(|e| new_cost < e.cost).unwrap_or(true);
                if new_cost_better {
                    explored.insert(
                        new_state.clone(),
                        Explored {
                            came_from: node.state.clone(),
                            cost: new_cost,
                            steps,
                        },
                    );
                    let score =
                        new_cost + self.remaining_distance_heuristic(info, &new_state, new_cost);
                    queue.push(ScheduleNode {
                        state: new_state,
                        cost: new_cost,
                        score,
                        at_end,
                    });
                }
            }
        }

        // TODO: Add actual "couldn't schedule error" because this is reachable if no solutions are
        // found because of stack too deep.
        unreachable!()
    }

    fn summarize(
        &mut self,
        info: ScheduleInfo,
        node: &ScheduleNode,
        steps: &Vec<Step>,
        queue: &ScheduleQueue,
        explored: &ExploredMap,
    ) -> Self::Summary;

    fn estimate_explored_map_size(
        &mut self,
        _info: ScheduleInfo,
        _start_state: &BackwardsMachine,
    ) -> usize {
        0
    }

    fn remaining_distance_heuristic(
        &mut self,
        _info: ScheduleInfo,
        _state: &BackwardsMachine,
        _cost: u32,
    ) -> u32 {
        0
    }

    #[inline]
    fn on_schedule_start(&mut self, _info: ScheduleInfo, _start_state: &BackwardsMachine) {}

    #[inline]
    fn on_explored_path(
        &mut self,
        _info: ScheduleInfo,
        _new_state: &BackwardsMachine,
        _new_cost: u32,
        _explored: &Option<&Explored>,
    ) {
    }
}
