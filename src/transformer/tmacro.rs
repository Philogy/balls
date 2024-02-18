use crate::comp_graph::{CompNode, CompNodeId, Computation};

#[derive(Debug, Clone)]
pub struct TransformedMacro {
    pub nodes: Vec<(CompNode, Computation)>,
    pub input_ids: Vec<CompNodeId>,
    pub output_ids: Vec<CompNodeId>,
    pub assignments: Vec<(String, CompNodeId)>,
    pub top_level_deps: Vec<CompNodeId>,
}

impl TransformedMacro {
    pub fn show_comps(&self) {
        self.nodes.iter().enumerate().for_each(|(id, (_, comp))| {
            println!("{:3>}: {:?}", id, comp);
        });
    }
}
