use crate::comp_graph::{CompNode, CompNodeId};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Action {
    Unpop(usize),
    Dedup(CompNodeId, usize),
    Undo(CompNodeId),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BackwardsMachine<'a> {
    all_nodes: &'a Vec<CompNode>,
    target_input_stack: &'a Vec<CompNodeId>,
    current_stack: Vec<CompNodeId>,
    undoable_effects: Vec<CompNodeId>,
}

impl<'a> BackwardsMachine<'a> {
    pub fn apply_action(&mut self, action: Action) {
        // match action {
        //     Unpop(idx)

        // }
    }
}
