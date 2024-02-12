use crate::comp_graph::{CompNode, CompNodeId, Computation};

#[derive(Debug, Clone)]
pub struct TransformedMacro {
    pub nodes: Vec<(CompNode, Computation)>,
    pub input_ids: Vec<CompNodeId>,
    pub output_ids: Vec<CompNodeId>,
    pub statement_ids: Vec<CompNodeId>,
    pub top_level_deps: Vec<CompNodeId>,
}
