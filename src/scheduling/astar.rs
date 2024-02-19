use super::actions::ActionIterator;
use crate::scheduling::swap::Swapper;
use crate::scheduling::{BackwardsMachine, ScheduleInfo, Step};
use crate::TimeDelta;
use std::collections::{BinaryHeap, HashMap};
use std::time::Instant;
use xxhash_rust::xxh3::Xxh3Builder;

#[derive(Debug, Clone)]
pub struct SchedulingTracker {
    start: Instant,
    total_time: f64,
    final_cost: u32,
    total_explored: usize,
    capacity_estimation: f64,
}

impl SchedulingTracker {
    pub fn record_end(&mut self, final_cost: u32, capacity_estimation: f64) {
        self.total_time = self.start.elapsed().as_secs_f64();
        self.final_cost = final_cost;
        self.capacity_estimation = capacity_estimation;
    }

    pub fn report(&self) {
        println!("\nScheduling: {}", self.total_time.humanize_seconds());
        println!(
            "explored: {} ({:.0} / s)",
            self.total_explored,
            self.total_explored as f64 / self.total_time
        );
        println!("cost (total SWAPs): {}", self.final_cost);
        let (is_pos, fmt_factor) = self.capacity_estimation.humanize_factor();
        if is_pos {
            println!("Overestimated explored nodes by: {}", fmt_factor);
        } else {
            println!("Underestimated explored nodes by: {}", fmt_factor);
        }
    }
}

impl Default for SchedulingTracker {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            total_time: 0.0,
            final_cost: 0,
            total_explored: 0,
            capacity_estimation: 0.0,
        }
    }
}

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

pub trait AStarScheduler: Sized {
    fn schedule(
        mut self,
        info: ScheduleInfo,
        start: BackwardsMachine,
    ) -> (Vec<Step>, SchedulingTracker) {
        let mut tracker = SchedulingTracker::default();

        let mut queue: ScheduleQueue = BinaryHeap::new();
        let est_capacity = self.estimate_explored_map_size(info, &start);
        let mut explored: ExploredMap =
            HashMap::with_capacity_and_hasher(est_capacity, Xxh3Builder::default());

        let score = self.estimate_remaining_cost(info, &start, 0);
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
                tracker.record_end(node.cost, est_capacity as f64 / explored.len() as f64);
                return (all_steps, tracker);
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
                tracker.total_explored += 1;
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
                    let score = new_cost + self.estimate_remaining_cost(info, &new_state, new_cost);
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

    fn estimate_explored_map_size(
        &mut self,
        _info: ScheduleInfo,
        _start_state: &BackwardsMachine,
    ) -> usize {
        0
    }

    fn estimate_remaining_cost(
        &mut self,
        _info: ScheduleInfo,
        _state: &BackwardsMachine,
        _cost: u32,
    ) -> u32;
}
