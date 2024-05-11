use crate::scheduling::actions::Action;
use crate::scheduling::ir::{CompNode, CompNodeId, IRGraph};
use crate::scheduling::Step;
use crate::scheduling::Swapper;
use crate::Searchable;

#[derive(Debug, Clone, Copy)]
pub struct ScheduleInfo<'a> {
    pub nodes: &'a [CompNode],
    pub target_input_stack: &'a [CompNodeId],
    pub variants: &'a [Option<Vec<usize>>],
}

impl<'a> From<&'a IRGraph> for ScheduleInfo<'a> {
    fn from(graph: &'a IRGraph) -> Self {
        Self {
            nodes: graph.nodes.as_slice(),
            target_input_stack: graph.input_ids.as_slice(),
            variants: graph.variants.as_slice(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
pub struct BackwardsMachine {
    pub stack: Vec<CompNodeId>,
    /// Amount of post dependencies and dependent contracts before the given node can be marked as
    /// "done".
    pub blocked_by: Vec<Option<u32>>,
}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for BackwardsMachine {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.stack
            .cmp(&other.stack)
            .then_with(|| self.blocked_by.cmp(&other.blocked_by))
    }
}

const MAX_VALID_SWAP_DEPTH: usize = 16;

impl BackwardsMachine {
    pub fn new(end_stack: Vec<CompNodeId>, blocked_by: Vec<Option<u32>>) -> Self {
        Self {
            stack: end_stack,
            blocked_by,
        }
    }

    pub fn all_done(&self) -> bool {
        self.blocked_by.iter().all(|b| b.is_none())
    }

    pub fn total_blocked(&self) -> u32 {
        self.blocked_by.iter().map(|b| b.unwrap_or(0)).sum::<u32>()
    }

    pub fn apply(
        &mut self,
        info: ScheduleInfo,
        action: Action,
        steps: &mut Vec<Step>,
    ) -> Result<bool, String> {
        match action {
            Action::Unpop(id) => self.unpop(info, id, steps),
            Action::UndoComp(id, stack_idx, undoing_as_variant) => {
                self.undo_comp(info, id, stack_idx, steps, undoing_as_variant)
            }
            Action::UndoEffect(id) => self.undo_effect(info, id, steps),
            Action::Dedup(as_top_idx, other_idx) => self.dedup(info, as_top_idx, other_idx, steps),
        };

        let at_end = self.all_done();
        if at_end {
            self.swap_to_target(info, steps)?;
        }
        Ok(at_end)
    }

    pub fn swap_to_target(
        &mut self,
        info: ScheduleInfo,
        steps: &mut Vec<Step>,
    ) -> Result<(), String> {
        if self.stack.is_empty() {
            debug_assert_eq!(
                info.target_input_stack.len(),
                0,
                "Ended with a stack of size 0 but target_input_stack has a non-zero length"
            );
        } else {
            let mut swapper = Swapper::new(&mut self.stack, info.target_input_stack);
            for depth in swapper.get_swaps() {
                if depth > MAX_VALID_SWAP_DEPTH {
                    return Err(format!("Invalid necessary swap depth: {}", depth));
                }
                steps.push(Step::Swap(depth));
            }
            assert!(
                swapper.matching_count().unwrap(),
                "Not-matching count according to swapper despite all_done => true"
            );
        }
        Ok(())
    }

    fn unpop(&mut self, info: ScheduleInfo, id: CompNodeId, steps: &mut Vec<Step>) {
        debug_assert_eq!(
            self.blocked_by[id],
            Some(0),
            "Unpopping blocked/done element (id: {})",
            id
        );
        // One unpop is sufficient to guarantee being done because every input stack element is
        // expected to be unique.
        if info.target_input_stack.contains(&id) {
            self.blocked_by[id] = None;
        }
        self.stack.push(id);

        steps.push(Step::Pop);
    }

    fn undo_comp(
        &mut self,
        info: ScheduleInfo,
        id: CompNodeId,
        stack_idx: usize,
        steps: &mut Vec<Step>,
        undoing_as_variant: bool,
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
        self._undo_node(info, id, undoing_as_variant);
        steps.push(Step::Comp(id, undoing_as_variant));
    }

    fn undo_effect(&mut self, info: ScheduleInfo, id: CompNodeId, steps: &mut Vec<Step>) {
        debug_assert!(
            !info.nodes[id].produces_value,
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
        self._undo_node(info, id, false);
        steps.push(Step::Comp(id, false));
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

    fn _undo_node(&mut self, info: ScheduleInfo, id: CompNodeId, undoing_as_variant: bool) {
        let mut push_to_stack = |dep_id: &usize| {
            self.stack.push(*dep_id);
            self.blocked_by[*dep_id].as_mut().map(|b| *b -= 1);
            // Can mark operand as "done" if its blocked count is 0 and we know it doesn't need to
            // be undone because it's on the target input stack.
            if self.blocked_by[*dep_id].unwrap() == 0 && info.target_input_stack.contains(dep_id) {
                debug_assert_eq!(self.stack.iter().total(dep_id), 1);
                self.blocked_by[*dep_id] = None;
            }
        };

        if undoing_as_variant {
            let variant = info.variants[id]
                .as_ref()
                .expect("undoing_as_variant flag without variant");
            let operands = &info.nodes[id].operands;
            variant
                .iter()
                .rev()
                .for_each(|op_index| push_to_stack(&operands[*op_index]));
        } else {
            info.nodes[id].operands.iter().rev().for_each(push_to_stack);
        }

        for pre_id in info.nodes[id].post.iter() {
            self.blocked_by[*pre_id].as_mut().map(|b| *b -= 1);
        }
    }
}
