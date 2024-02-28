use super::actions::get_actions;
use crate::scheduling::ir::IRGraph;
use crate::scheduling::{BackwardsMachine, ScheduleInfo, Step};
use crate::CommaSeparatable;
use crate::TimeDelta;
use std::collections::{BinaryHeap, HashMap};
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::time::Instant;
use xxhash_rust::xxh3::{Xxh3, Xxh3Builder};

#[derive(Debug, Clone)]
pub struct SchedulingTracker {
    start: Instant,
    total_time: f64,
    final_cost: u32,
    total_explored: usize,
    total_collisions: usize,
    capacity_estimation: (usize, usize),
}

impl SchedulingTracker {
    pub fn record_end(&mut self, final_cost: u32, capacity_estimate: usize, final_capacity: usize) {
        self.total_time = self.start.elapsed().as_secs_f64();
        self.final_cost = final_cost;
        self.capacity_estimation = (capacity_estimate, final_capacity);
    }

    pub fn report(&self, indent: usize) {
        let indent = " ".repeat(indent);
        println!(
            "{}Scheduling: {}",
            indent,
            self.total_time.humanize_seconds()
        );
        println!(
            "{}explored: {} ({} / s)",
            indent,
            self.total_explored.comma_sep(),
            ((self.total_explored as f64 / self.total_time).round() as usize).comma_sep()
        );
        println!("{}cost (total SWAPs): {}", indent, self.final_cost);
        let (capacity_estimate, final_capacity) = self.capacity_estimation;
        if capacity_estimate == 0 {
            println!(
                "{}Final explored capacity (estimated 0): {}",
                indent,
                final_capacity.comma_sep()
            );
        } else {
            let off_factor = capacity_estimate as f64 / final_capacity as f64;
            let (is_pos, fmt_factor) = off_factor.humanize_factor();
            if is_pos {
                println!(
                    "{}Overestimated explored nodes by: {} (est: {} vs. final: {})",
                    indent,
                    fmt_factor,
                    capacity_estimate.comma_sep(),
                    final_capacity.comma_sep()
                );
            } else {
                println!(
                    "{}Underestimated explored nodes by: {} (est: {} vs. final: {})",
                    indent,
                    fmt_factor,
                    capacity_estimate.comma_sep(),
                    final_capacity.comma_sep()
                );
            }
        }
        println!(
            "{}Overwritten explored: {} ({:.2}%)",
            indent,
            self.total_collisions.comma_sep(),
            self.total_collisions as f32 / final_capacity as f32 * 100.0
        );
    }
}

impl Default for SchedulingTracker {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            total_time: 0.0,
            final_cost: 0,
            total_explored: 0,
            total_collisions: 0,
            capacity_estimation: (0, 0),
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
    came_from: u64,
    steps: Vec<Step>,
    cost: u32,
}

type ExploredMap = HashMap<u64, Explored, BuildHasherDefault<NoopHasher>>;
type ScheduleQueue = BinaryHeap<ScheduleNode>;

struct FastHasher(Xxh3);

impl FastHasher {
    fn new() -> Self {
        Self(Xxh3Builder::new().build())
    }

    fn hash_one_off<T: Hash>(&mut self, value: &T) -> u64 {
        value.hash(&mut self.0);
        let hash = self.0.finish();
        self.0.reset();
        hash
    }
}

#[derive(Clone, Debug, Default)]
struct NoopHasher(u64);

impl Hasher for NoopHasher {
    fn write(&mut self, _bytes: &[u8]) {
        todo!()
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

pub trait AStarScheduler: Sized {
    fn schedule(
        mut self,
        graph: &IRGraph,
        max_stack_depth: usize,
    ) -> (Vec<Step>, SchedulingTracker) {
        let mut tracker = SchedulingTracker::default();

        let mut queue: ScheduleQueue = BinaryHeap::new();
        let info = ScheduleInfo {
            nodes: graph.nodes.as_slice(),
            target_input_stack: graph.input_ids.as_slice(),
        };
        let start = BackwardsMachine::new(
            graph.output_ids.clone(),
            graph.nodes.iter().map(|node| node.blocked_by).collect(),
        );
        let est_capacity = self.estimate_explored_map_size(info, &start);
        let mut explored: ExploredMap =
            HashMap::with_capacity_and_hasher(est_capacity, Default::default());

        let score = self.estimate_remaining_cost(info, &start, 0);
        queue.push(ScheduleNode {
            state: start.clone(),
            cost: 0,
            score,
            at_end: start.all_done(),
        });

        let mut hasher = FastHasher::new();

        // 1. Pop top of priority queue (node closest to end according to actual cost + estimated
        //    remaining distance).
        while let Some(node) = queue.pop() {
            let came_from = hasher.hash_one_off(&node.state);
            // 2a. If the shortest node is the end we know we found our solution, accumulate the
            // steps and return.
            if node.at_end {
                let mut state_key = came_from;
                let mut all_steps = vec![];
                while let Some(e) = explored.remove(&state_key) {
                    all_steps.extend(e.steps.into_iter().rev());
                    state_key = e.came_from;
                }

                let explored_size = explored.len();

                // TODO: Use arena allocator to be able to more efficiently drop the allocations.
                // Degen: Purposefully leak the data structures as it takes *a lot* of time to
                // properly drop and clear.
                std::mem::forget(explored);
                std::mem::forget(queue);

                tracker.record_end(node.cost, est_capacity, explored_size);
                return (all_steps, tracker);
            }

            // 2b. Not at the end so we explore all possible neighbours.
            for action in get_actions(info, &node.state) {
                let mut new_state = node.state.clone();
                let mut steps = vec![];
                let at_end = new_state.apply(info, action, &mut steps).unwrap();
                if new_state.stack.len() > max_stack_depth {
                    continue;
                }
                let new_cost = node.cost + steps.iter().map(|step| step.cost()).sum::<u32>();
                tracker.total_explored += 1;
                let new_state_hash = hasher.hash_one_off(&new_state);
                let new_cost_better = match explored.get(&new_state_hash) {
                    Some(e) => new_cost < e.cost,
                    None => true,
                };
                if new_cost_better {
                    let out = explored.insert(
                        new_state_hash,
                        Explored {
                            came_from,
                            cost: new_cost,
                            steps,
                        },
                    );
                    tracker.total_collisions += if out.is_some() { 1 } else { 0 };
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

        panic!("TODO: Impossible to schedule within specified bounds (likely stack-too-deep).")
    }

    fn estimate_explored_map_size(
        &mut self,
        _info: ScheduleInfo,
        start_state: &BackwardsMachine,
    ) -> usize {
        let blocks = start_state.total_blocked() as usize;
        blocks * blocks * 30
    }

    fn estimate_remaining_cost(
        &mut self,
        _info: ScheduleInfo,
        _state: &BackwardsMachine,
        _cost: u32,
    ) -> u32;
}
