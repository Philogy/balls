use crate::comp_graph::{CompNode, CompNodeId};
use crate::schedulers::Step;
use crate::transformer::TransformedMacro;
use crate::Searchable;
use std::rc::Rc;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    Unpop(CompNodeId),
    Dedup(CompNodeId, usize),
    UndoEffect(CompNodeId, usize),
    UndoComp(CompNodeId, usize),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackwardsMachine {
    all_nodes: Rc<[CompNode]>,
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
    pub fn new(
        all_nodes: Rc<[CompNode]>,
        target_input_stack: Rc<[CompNodeId]>,
        stack: Vec<CompNodeId>,
        blocked_by: Vec<u32>,
    ) -> Self {
        let total_nodes = all_nodes.len();
        let done = (0..total_nodes)
            .map(|id| {
                blocked_by[id] == 0
                    && stack.iter().total(&id) == target_input_stack.iter().total(&id)
            })
            .collect();
        Self {
            all_nodes,
            target_input_stack,
            stack,
            steps: vec![],
            blocked_by,
            done,
        }
    }

    pub fn is_on_stack(&self, id: CompNodeId) -> bool {
        self.stack.iter().contains(&id)
    }

    pub fn is_input(&self, id: CompNodeId) -> bool {
        self.target_input_stack.iter().contains(&id)
    }

    pub fn can_undo(&self, id: CompNodeId) -> bool {
        if self.done[id] || self.blocked_by[id] > 0 {
            false
        } else if !self.all_nodes[id].has_output {
            true
        } else {
            !self.is_input(id)
        }
    }

    pub fn apply<'b>(&mut self, action: Action) {
        match action {
            Action::Unpop(id) => self.unpop(id),
            Action::UndoComp(id, stack_idx) => self.undo_comp(id, stack_idx),
            Action::UndoEffect(id, stack_idx) => self.undo_effect(id, stack_idx),
            _ => todo!(),
        }
    }

    fn unpop(&mut self, id: CompNodeId) {
        debug_assert_eq!(self.blocked_by[id], 0, "Unpopping blocked element {}", id);
        debug_assert!(!self.done[id], "Unpopping done element {}", id);
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
            "Id stack index mismatch depth: {}, passed: {}, actual: {}",
            depth, id, actual_id
        );

        if depth > 0 {
            self.steps.push(Step::Swap(depth));
        }

        self.steps.push(Step::Op(id));

        for operand_id in self.all_nodes[id].operands.iter().rev() {
            self.stack.push(*operand_id);
            self.blocked_by[*operand_id] -= 1;
        }
        for pre_id in self.all_nodes[id].post.iter() {
            let new_blocked_by = self.blocked_by[*pre_id] - 1;
            self.blocked_by[*pre_id] = new_blocked_by;
        }
    }

    fn undo_effect(&mut self, id: CompNodeId, stack_idx: usize) {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unpop() {
        let mut machine = BackwardsMachine {
            all_nodes: Rc::from([
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
            all_nodes: Rc::from([
                CompNode::lone(0, true),
                CompNode::lone(1, true),
                CompNode::lone(2, true),
                CompNode::new(3, true, vec![0, 1], vec![]),
            ]),
            target_input_stack: Rc::from([2, 1, 0]),
            stack: vec![3, 2],
            steps: vec![],
            blocked_by: vec![1, 1, 0, 0],
            done: vec![false; 4],
        };

        dbg!(&machine);

        machine.apply(Action::UndoComp(3, 0));

        dbg!(&machine);
    }
}

impl From<TransformedMacro> for BackwardsMachine {
    fn from(tmacro: TransformedMacro) -> Self {
        let only_nodes: Vec<_> = tmacro.nodes.into_iter().map(|(node, _)| node).collect();
        let all_nodes: Rc<[CompNode]> = Rc::from(only_nodes);
        let target_input_stack = Rc::from(tmacro.input_ids);

        let mut blocked_by = vec![0u32; all_nodes.len()];
        let mut stack_count = vec![0u32; all_nodes.len()];

        for node in all_nodes.iter() {
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

        for id in 0..all_nodes.len() {
            let required_dedups = stack_count[id].max(1) - 1;
            blocked_by[id] += required_dedups;
        }

        BackwardsMachine::new(all_nodes, target_input_stack, stack, blocked_by)
    }
}
