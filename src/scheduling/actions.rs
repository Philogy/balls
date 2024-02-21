use crate::comp_graph::CompNodeId;
use crate::scheduling::{BackwardsMachine, ScheduleInfo};
use crate::Searchable;
use std::vec::IntoIter;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    Unpop(CompNodeId),
    Dedup(usize, usize),
    UndoEffect(CompNodeId),
    UndoComp(CompNodeId, usize),
}

pub struct ActionIterator(IntoIter<Action>);

impl ActionIterator {
    pub fn new(info: ScheduleInfo, machine: &BackwardsMachine) -> Self {
        let mut actions = vec![];

        let unpoppable: Vec<_> = (0..info.nodes.len())
            .filter(|id| {
                machine.blocked_by[*id] == Some(0)
                    && info.nodes[*id].has_output
                    && !machine.stack.contains(id)
            })
            .collect();

        let stack = &machine.stack;
        let total_stack_el = stack.len();
        let deepest_idx = total_stack_el.checked_sub(17).unwrap_or(0);

        actions.extend((deepest_idx..total_stack_el).filter_map(|i| {
            (deepest_idx..total_stack_el).find_map(|j| {
                if i != j && stack[i] == stack[j] {
                    Some(Action::Dedup(i, j))
                } else {
                    None
                }
            })
        }));

        actions.extend((0..info.nodes.len()).filter_map(|id| {
            if machine.blocked_by[id] != Some(0) || unpoppable.contains(&id) {
                return None;
            }
            if info.nodes[id].has_output {
                let idx = machine.stack.iter().index_of(&id).expect(
                    format!(
                        "Not-yet-done, comp node with 0 blocks not on stack (id: {}, stack: {:?})",
                        id, stack
                    )
                    .as_str(),
                );
                if idx < deepest_idx {
                    None
                } else {
                    Some(Action::UndoComp(id, idx))
                }
            } else {
                Some(Action::UndoEffect(id))
            }
        }));

        actions.extend(unpoppable.into_iter().map(Action::Unpop));

        Self(actions.into_iter())
    }
}

impl Iterator for ActionIterator {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
