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

        actions.extend(
            (0..machine.nodes.len())
                .filter(|id| machine.blocked_by(*id) == Some(0))
                .map(|id| {
                    if machine.nodes[id].has_output {
                        Action::UndoComp(
                            id,
                            machine
                                .stack()
                                .iter()
                                .index_of(&id)
                                .expect("Not-done, 0 block comp not on stack???"),
                        )
                    } else {
                        Action::UndoEffect(id)
                    }
                }),
        );

        Self(actions.into_iter())
    }
}

impl Iterator for ActionIterator {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}
