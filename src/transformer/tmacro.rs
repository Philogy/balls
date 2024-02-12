use crate::comp_graph::{CompNode, CompNodeId, Computation};
use crate::schedulers::BackwardsMachine;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct TransformedMacro {
    pub nodes: Vec<(CompNode, Computation)>,
    pub input_ids: Vec<CompNodeId>,
    pub output_ids: Vec<CompNodeId>,
    pub statement_ids: Vec<CompNodeId>,
    pub top_level_deps: Vec<CompNodeId>,
}
