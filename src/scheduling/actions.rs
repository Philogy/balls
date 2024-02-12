use crate::comp_graph::CompNodeId;
use crate::scheduling::BackwardsMachine;
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
    pub fn new(machine: &BackwardsMachine) -> Self {
        let mut actions = vec![];

        actions.extend(
            machine
                .target_input_stack
                .iter()
                .map(|id| *id)
                .filter(|id| machine.blocked_by(*id) == Some(0))
                .map(Action::Unpop),
        );

        let stack = machine.stack();
        let total_stack_el = stack.len();

        for i in 0..total_stack_el {
            actions.extend((0..total_stack_el).filter_map(|j| {
                if i == j {
                    return None;
                }
                let id1 = stack[i];
                let id2 = stack[j];
                if id1 != id2 || machine.blocked_by(id1).unwrap_or(0) == 0 {
                    return None;
                }
                // We know length is at least 2 because we have 2 distinct indices (i, j)
                let last_idx = total_stack_el - 1;
                // Skip, too deep.
                if last_idx - i > 16 || last_idx - j > 16 {
                    return None;
                }
                Some(Action::Dedup(i, j))
            }));
        }

        actions.extend((0..machine.nodes.len()).filter_map(|id| {
            if machine.blocked_by(id) != Some(0) {
                return None;
            }
            if machine.nodes[id].has_output {
                Some(Action::UndoComp(
                    id,
                    machine
                        .stack()
                        .iter()
                        .index_of(&id)
                        .expect("Not-done, 0 block comp not on stack???"),
                ))
            } else {
                Some(Action::UndoEffect(id))
            }
        }));

        Self(actions.into_iter())
    }
}

impl Iterator for ActionIterator {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
