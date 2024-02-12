use crate::scheduling::actions::Action;
use crate::scheduling::swap::Swapper;
use crate::scheduling::{BackwardsMachine, Step};
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ScheduleNode {
    state: BackwardsMachine,
    /// Real, known cost.
    cost: u32,
    /// Total cost (including heuristic).
    score: u32,
}

impl Ord for ScheduleNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .score
            .cmp(&self.score)
            .then(other.cost.cmp(&self.cost))
    }
}

impl PartialOrd for ScheduleNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive()]
pub struct Explored {
    came_from: BackwardsMachine,
    steps: Vec<Step>,
    cost: u32,
}

fn show(node: &ScheduleNode) {
    println!("  stack: {:?}", node.state.stack());
    println!("  blocked: {:?}", node.state.blocked_by);
}

pub trait AStarScheduler<A>
where
    A: Iterator<Item = Action>,
{
    fn schedule(start: BackwardsMachine) -> Vec<Step> {
        let mut queue: BinaryHeap<ScheduleNode> = BinaryHeap::new();
        let mut explored: HashMap<BackwardsMachine, Explored> = HashMap::new();
        let mut total = 0;

        let score = Self::remaining_distance_heuristic(&start);
        queue.push(ScheduleNode {
            state: start.clone(),
            cost: 0,
            score,
        });

        while let Some(node) = queue.pop() {
            // println!("popped node:");
            // show(&node);
            if node.state.all_done() {
                let mut state_key = &node.state;
                let mut all_steps = vec![];
                while let Some(e) = explored.get(state_key) {
                    all_steps.extend(e.steps.clone().into_iter().rev());
                    state_key = &e.came_from;
                }
                println!("total: {}", total);
                return all_steps;
            }
            for action in Self::iter_actions(&node.state) {
                let mut new_state = node.state.clone();
                let mut steps = vec![];
                new_state.apply(action, &mut steps);
                if new_state.all_done() {
                    let mut swapper =
                        Swapper::new(&mut new_state.stack, &start.target_input_stack[..]);
                    match swapper.get_swaps() {
                        Ok(s) => steps.extend(s.into_iter().map(Step::Swap)),
                        Err(_) => continue,
                    }
                    assert!(
                        swapper.matching_count().unwrap(),
                        "Not-matching count according to swapper despite all_done => true"
                    );
                }
                total += 1;
                let new_cost = node.cost + steps.iter().map(|step| step.cost()).sum::<u32>();
                let e = explored.get(&new_state);
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
                    let score = new_cost + Self::remaining_distance_heuristic(&new_state);
                    queue.push(ScheduleNode {
                        state: new_state,
                        cost: new_cost,
                        score,
                    });
                }
            }
        }

        // TODO: Add actual "couldn't schedule error" because this is reachable if all paths lead
        // to stack too deep.
        unreachable!()
    }

    fn remaining_distance_heuristic(_state: &BackwardsMachine) -> u32 {
        0
    }

    fn iter_actions(state: &BackwardsMachine) -> A;
}
