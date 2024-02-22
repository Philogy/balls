use crate::comp_graph::CompNodeId;
use crate::scheduling::{BackwardsMachine, ScheduleInfo};
use crate::Searchable;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    Unpop(CompNodeId),
    Dedup(usize, usize),
    UndoEffect(CompNodeId),
    UndoComp(CompNodeId, usize),
}

pub fn get_actions<'a>(
    info: ScheduleInfo<'a>,
    machine: &'a BackwardsMachine,
) -> impl Iterator<Item = Action> + 'a {
    let unpoppable: Vec<_> = (0..info.nodes.len())
        .filter(|id| {
            machine.blocked_by[*id] == Some(0)
                && info.nodes[*id].has_output
                && !machine.stack.contains(id)
        })
        .collect();

    let total_stack_el = machine.stack.len();
    let deepest_idx = total_stack_el.checked_sub(17).unwrap_or(0);

    (deepest_idx..total_stack_el)
        .filter_map(move |i| {
            (deepest_idx..total_stack_el).find_map(|j| {
                if i != j && machine.stack[i] == machine.stack[j] {
                    Some(Action::Dedup(i, j))
                } else {
                    None
                }
            })
        })
        .chain(unpoppable.clone().into_iter().map(Action::Unpop))
        .chain((0..info.nodes.len()).filter_map(move |id| {
            if machine.blocked_by[id] != Some(0) || unpoppable.contains(&id) {
                return None;
            }
            if info.nodes[id].has_output {
                let idx = machine.stack.iter().index_of(&id).unwrap_or_else(|| {
                    panic!(
                        "Not-yet-done, comp node with 0 blocks not on stack (id: {}, stack: {:?})",
                        id, machine.stack
                    )
                });
                if idx < machine.stack.len().checked_sub(17).unwrap_or(0) {
                    None
                } else {
                    Some(Action::UndoComp(id, idx))
                }
            } else {
                Some(Action::UndoEffect(id))
            }
        }))
}
