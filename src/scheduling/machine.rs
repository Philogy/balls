use crate::comp_graph::{CompNode, CompNodeId};
use crate::scheduling::Step;
use crate::transformer::TransformedMacro;
use crate::Searchable;
use std::rc::Rc;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    Unpop(CompNodeId),
    Dedup(CompNodeId, usize),
    UndoEffect(CompNodeId),
    UndoComp(usize, usize),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackwardsMachine {
    nodes: Rc<[CompNode]>,
    target_input_stack: Rc<[CompNodeId]>,
    stack: Vec<CompNodeId>,
    /// Executed steps (in undone order).
    steps: Vec<Step>,
    /// Amount of post dependencies and dependent contracts before the given node can be marked as
    /// "done".
    blocked_by: Vec<u32>,
    done: Vec<bool>,
}

impl BackwardsMachine {
    pub fn is_on_stack(&self, id: CompNodeId) -> bool {
        self.stack.iter().contains(&id)
    }

    pub fn is_input(&self, id: CompNodeId) -> bool {
        self.target_input_stack.iter().contains(&id)
    }

    pub fn can_undo(&self, id: CompNodeId) -> bool {
        if self.done[id] || self.blocked_by[id] > 0 {
            false
        } else if !self.nodes[id].has_output {
            true
        } else {
            !self.is_input(id)
        }
    }

    pub fn apply<'b>(&mut self, action: Action) {
        match action {
            Action::Unpop(id) => self.unpop(id),
            Action::UndoComp(id, stack_idx) => self.undo_comp(id, stack_idx),
            Action::UndoEffect(id) => self.undo_effect(id),
            Action::Dedup(as_top_idx, other_idx) => self.dedup(as_top_idx, other_idx),
        }
    }

    fn unpop(&mut self, id: CompNodeId) {
        debug_assert_eq!(
            self.blocked_by[id], 0,
            "Unpopping blocked element (id: {})",
            id
        );
        debug_assert!(!self.done[id], "Unpopping done element (id: {})", id);
        debug_assert!(
            self.target_input_stack.contains(&id),
            "Unpopping element not in input stack (id: {})",
            id
        );
        // One unpop is sufficient to guarantee being done because every input stack element is
        // expected to be unique.
        self.done[id] = true;
        self.stack.push(id);
        self.steps.push(Step::Pop);
    }

    fn undo_comp(&mut self, id: CompNodeId, stack_idx: usize) {
        // Check that we're actually able to undo.
        debug_assert_eq!(self.blocked_by[id], 0, "Undoing blocked element {}", id);
        debug_assert!(!self.done[id], "Unpopping done element {}", id);
        self.done[id] = true;

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
            self.steps.push(Step::Swap(depth));
        }

        self._undo_node(id);
    }

    fn undo_effect(&mut self, id: CompNodeId) {
        debug_assert!(
            !self.nodes[id].has_output,
            "Attempting to undo node as effect which has output (id: {})",
            id
        );
        debug_assert_eq!(self.blocked_by[id], 0, "Undoing blocked element {}", id);
        debug_assert!(!self.done[id], "Unpopping done element {}", id);
        self.done[id] = true;
        self._undo_node(id);
    }

    fn dedup(&mut self, as_top_idx: usize, other_idx: usize) {
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
        debug_assert!(!self.done[id], "Deduping done element (id: {})", id);
        debug_assert!(
            self.blocked_by[id] > 0,
            "Deduping element with no blocks (id: {})",
            id
        );
        let top_idx = self.stack.len() - 1;
        let (as_top_idx, other_idx) = if as_top_idx > other_idx {
            (as_top_idx, other_idx)
        } else {
            (other_idx, as_top_idx)
        };
        debug_assert!(
            as_top_idx <= top_idx,
            "Dedup (at top) index out-of-bounds: {}",
            top_idx
        );
        debug_assert!(
            other_idx <= top_idx,
            "Dedup (other) index out-of-bounds: {}",
            other_idx
        );
        let swap_depth = top_idx - as_top_idx;
        if swap_depth > 0 {
            self.steps.push(Step::Swap(swap_depth));
            self.stack.swap(as_top_idx, top_idx);
        }
        let dedup_depth = top_idx - other_idx;
        debug_assert!(
            dedup_depth <= 16,
            "Balls too deep (attempted to dedup with depth: {})",
            dedup_depth
        );
        self.steps.push(Step::Dup(dedup_depth));
        self.stack.pop();
    }

    fn _undo_node(&mut self, id: CompNodeId) {
        self.steps.push(Step::Op(id));
        for dep_id in self.nodes[id].operands.iter().rev() {
            self.stack.push(*dep_id);
            self.blocked_by[*dep_id] -= 1;
            if self.blocked_by[*dep_id] == 0 && self.target_input_stack.contains(dep_id) {
                debug_assert_eq!(self.stack.iter().total(dep_id), 1);
                self.done[*dep_id] = true;
            }
        }
        for pre_id in self.nodes[id].post.iter() {
            self.blocked_by[*pre_id] -= 1;
        }
    }
}

impl From<TransformedMacro> for BackwardsMachine {
    fn from(tmacro: TransformedMacro) -> Self {
        let only_nodes: Vec<_> = tmacro.nodes.into_iter().map(|(node, _)| node).collect();
        let nodes: Rc<[CompNode]> = Rc::from(only_nodes);
        let target_input_stack: Rc<[CompNodeId]> = Rc::from(tmacro.input_ids);

        let mut blocked_by = vec![0u32; nodes.len()];
        let mut stack_count = vec![0u32; nodes.len()];

        for node in nodes.iter() {
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

        for id in 0..nodes.len() {
            let required_dedups = stack_count[id].max(1) - 1;
            blocked_by[id] += required_dedups;
        }

        let done = (0..nodes.len())
            .map(|id| {
                let input_count = target_input_stack.iter().total(&id);
                let already_done = blocked_by[id] == 0
                    && stack.iter().total(&id) == input_count
                    && !tmacro.top_level_deps.contains(&id);

                if already_done && input_count == 0 {
                    println!("TODO: Warning skipping scheduling for unused node {}", id);
                }

                already_done
            })
            .collect();

        Self {
            nodes,
            target_input_stack,
            stack,
            steps: vec![],
            blocked_by,
            done,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unpop() {
        let mut machine = BackwardsMachine {
            nodes: Rc::from([
                CompNode::lone(0, true),
                CompNode::lone(1, true),
                CompNode::lone(2, true),
                CompNode::new(3, true, vec![0, 1], vec![]),
            ]),
            target_input_stack: Rc::from([2, 1, 0]),
            stack: vec![3],
            steps: vec![],
            blocked_by: vec![1, 1, 0, 0],
            done: vec![false; 4],
        };

        dbg!(&machine);

        machine.apply(Action::Unpop(2));

        dbg!(&machine);
    }

    #[test]
    fn test_undo_comp() {
        let mut machine = BackwardsMachine {
            nodes: Rc::from([
                CompNode::lone(0, true),
                CompNode::lone(1, true),
                CompNode::lone(2, true),
                CompNode::new(3, true, vec![0, 1], vec![]),
            ]),
            target_input_stack: Rc::from([2, 1, 0]),
            stack: vec![2, 3],
            steps: vec![],
            blocked_by: vec![1, 1, 0, 0],
            done: vec![false, false, true, false],
        };

        dbg!(&machine);

        machine.apply(Action::UndoComp(3, 1));

        dbg!(&machine);
    }
}
