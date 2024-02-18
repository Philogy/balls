use crate::comp_graph::{CompNode, CompNodeId};
use crate::scheduling::actions::Action;
use crate::scheduling::Step;
use crate::transformer::TransformedMacro;
use crate::Searchable;

#[derive(Debug, Clone, Copy)]
pub struct ScheduleInfo<'a> {
    pub nodes: &'a [CompNode],
    pub target_input_stack: &'a [CompNodeId],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackwardsMachine {
    pub stack: Vec<CompNodeId>,
    /// Amount of post dependencies and dependent contracts before the given node can be marked as
    /// "done".
    pub blocked_by: Vec<Option<u32>>,
}

impl BackwardsMachine {
    pub fn all_done(&self) -> bool {
        self.blocked_by.iter().all(|b| b.is_none())
    }

    pub fn stack(&self) -> &Vec<CompNodeId> {
        &self.stack
    }

    pub fn apply(&mut self, info: ScheduleInfo, action: Action, steps: &mut Vec<Step>) {
        match action {
            Action::Unpop(id) => self.unpop(info, id, steps),
            Action::UndoComp(id, stack_idx) => self.undo_comp(info, id, stack_idx, steps),
            Action::UndoEffect(id) => self.undo_effect(info, id, steps),
            Action::Dedup(as_top_idx, other_idx) => self.dedup(info, as_top_idx, other_idx, steps),
        }
    }

    fn unpop(&mut self, info: ScheduleInfo, id: CompNodeId, steps: &mut Vec<Step>) {
        debug_assert_eq!(
            self.blocked_by[id],
            Some(0),
            "Unpopping blocked/done element (id: {})",
            id
        );
        debug_assert!(
            info.target_input_stack.contains(&id),
            "Unpopping element not in input stack (id: {})",
            id
        );
        // One unpop is sufficient to guarantee being done because every input stack element is
        // expected to be unique.
        self.blocked_by[id] = None;
        self.stack.push(id);

        steps.push(Step::Pop);
    }

    fn undo_comp(
        &mut self,
        info: ScheduleInfo,
        id: CompNodeId,
        stack_idx: usize,
        steps: &mut Vec<Step>,
    ) {
        // Check that we're actually able to undo.
        debug_assert_eq!(
            self.blocked_by[id],
            Some(0),
            "Undoing blocked/done element {}",
            id
        );
        self.blocked_by[id] = None;

        let last_stack_idx = self.stack.len() - 1;
        debug_assert!(
            stack_idx <= last_stack_idx,
            "Invalid stack index {}",
            last_stack_idx
        );
        let depth = last_stack_idx - stack_idx;
        debug_assert!(depth <= 16, "Balls too deep {}", depth);

        let actual_id = self.stack.swap_remove(stack_idx);
        debug_assert_eq!(
            actual_id, id,
            "Id stack index mismatch depth: {}, passed id: {}, actual id: {}",
            depth, id, actual_id
        );

        if depth > 0 {
            steps.push(Step::Swap(depth));
        }
        self._undo_node(info, id);
        steps.push(Step::Op(id));
    }

    fn undo_effect(&mut self, info: ScheduleInfo, id: CompNodeId, steps: &mut Vec<Step>) {
        debug_assert!(
            !info.nodes[id].has_output,
            "Attempting to undo node as effect which has output (id: {})",
            id
        );
        debug_assert_eq!(
            self.blocked_by[id],
            Some(0),
            "Undoing blocked/done element {}",
            id
        );
        self.blocked_by[id] = None;
        self._undo_node(info, id);
        steps.push(Step::Op(id));
    }

    fn dedup(
        &mut self,
        info: ScheduleInfo,
        as_top_idx: usize,
        other_idx: usize,
        steps: &mut Vec<Step>,
    ) {
        debug_assert!(
            as_top_idx != other_idx,
            "Duplicate indices for dedup (idx: {})",
            as_top_idx
        );
        let id = self.stack[as_top_idx];
        debug_assert_eq!(
            id, self.stack[other_idx],
            "IDs at indices don't match ([{}], [{}] -> {} vs. {})",
            as_top_idx, other_idx, id, self.stack[other_idx]
        );
        debug_assert!(
            self.blocked_by[id].unwrap_or(0) > 0,
            "Deduping element with no blocks/done (id: {})",
            id
        );
        let top_idx = self.stack.len() - 1;
        debug_assert!(
            as_top_idx <= top_idx,
            "Dedup index out-of-bounds: {}",
            top_idx
        );
        debug_assert!(
            other_idx <= top_idx,
            "Dedup (other) index out-of-bounds: {}",
            other_idx
        );
        let swap_depth = top_idx - as_top_idx;
        debug_assert!(
            swap_depth <= 16,
            "Balls too deep (attempted to swap with depth: {})",
            swap_depth
        );
        if swap_depth > 0 {
            steps.push(Step::Swap(swap_depth));
            self.stack.swap(as_top_idx, top_idx);
        }
        let dedup_depth = top_idx - other_idx;
        debug_assert!(
            dedup_depth <= 16,
            "Balls too deep (attempted to dedup with depth: {})",
            dedup_depth
        );
        self.blocked_by[id].as_mut().map(|b| *b -= 1);
        steps.push(Step::Dup(dedup_depth));
        self.stack.pop();
        if self.blocked_by[id].unwrap() == 0 && info.target_input_stack.contains(&id) {
            debug_assert_eq!(
                self.stack.iter().total(&id),
                1,
                "stack: {:?}, id: {}, ({}, {})",
                &self.stack,
                id,
                as_top_idx,
                other_idx
            );
            self.blocked_by[id] = None;
        }
    }

    fn _undo_node(&mut self, info: ScheduleInfo, id: CompNodeId) {
        for dep_id in info.nodes[id].operands.iter().rev() {
            self.stack.push(*dep_id);
            self.blocked_by[*dep_id].as_mut().map(|b| *b -= 1);
            if self.blocked_by[*dep_id].unwrap() == 0 && info.target_input_stack.contains(dep_id) {
                debug_assert_eq!(self.stack.iter().total(dep_id), 1);
                self.blocked_by[*dep_id] = None;
            }
        }
        for pre_id in info.nodes[id].post.iter() {
            self.blocked_by[*pre_id].as_mut().map(|b| *b -= 1);
        }
    }
}

impl From<TransformedMacro> for BackwardsMachine {
    fn from(tmacro: TransformedMacro) -> Self {
        let nodes = tmacro.nodes.len();

        let mut blocked_by = vec![0u32; nodes];
        let mut stack_count = vec![0u32; nodes];

        for (node, _) in tmacro.nodes.iter() {
            for post_id in node.post.iter() {
                blocked_by[*post_id] += 1;
            }
            for dep_id in node.operands.iter() {
                // Blocked once as an argument.
                blocked_by[*dep_id] += 1;
                stack_count[*dep_id] += 1;
            }
        }

        for output_id in tmacro.output_ids.iter() {
            stack_count[*output_id] += 1;
        }

        let stack = tmacro.output_ids;

        for id in 0..nodes {
            let required_dedups = stack_count[id].max(1) - 1;
            blocked_by[id] += required_dedups;
        }

        let blocked_by = (0..nodes)
            .map(|id| {
                let input_count = tmacro.input_ids.iter().total(&id);
                let already_done = blocked_by[id] == 0
                    && stack.iter().total(&id) == input_count
                    && !tmacro.top_level_deps.contains(&id);

                if already_done && input_count == 0 {
                    println!("TODO: Warning skipping scheduling for unused node {}", id);
                }

                if already_done {
                    None
                } else {
                    Some(blocked_by[id])
                }
            })
            .collect();

        Self { stack, blocked_by }
    }
}

impl std::hash::Hash for BackwardsMachine {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.stack.hash(state);
        self.blocked_by.hash(state);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unpop() {
        let nodes = vec![
            CompNode::lone(0, true),
            CompNode::lone(1, true),
            CompNode::lone(2, true),
            CompNode::new(3, true, vec![0, 1], vec![]),
        ];
        let target_input_stack = vec![2, 1, 0];
        let info = ScheduleInfo {
            nodes: nodes.as_slice(),
            target_input_stack: target_input_stack.as_slice(),
        };
        let mut machine = BackwardsMachine {
            stack: vec![3],
            blocked_by: vec![Some(1), Some(1), Some(0), Some(0)],
        };

        dbg!(&machine);

        machine.apply(info, Action::Unpop(2), &mut vec![]);

        dbg!(&machine);
    }

    #[test]
    fn test_undo_comp() {
        let nodes = vec![
            CompNode::lone(0, true),
            CompNode::lone(1, true),
            CompNode::lone(2, true),
            CompNode::new(3, true, vec![0, 1], vec![]),
        ];
        let target_input_stack = vec![2, 1, 0];
        let info = ScheduleInfo {
            nodes: nodes.as_slice(),
            target_input_stack: target_input_stack.as_slice(),
        };
        let mut machine = BackwardsMachine {
            stack: vec![2, 3],
            blocked_by: vec![Some(1), Some(1), Some(0), Some(0)],
        };

        dbg!(&machine);

        machine.apply(info, Action::UndoComp(3, 1), &mut vec![]);

        dbg!(&machine);
    }
}
