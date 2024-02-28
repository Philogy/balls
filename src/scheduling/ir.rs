pub type CompNodeId = usize;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CompNode {
    pub blocked_by: Option<u32>,
    pub produces_value: bool,
    pub operands: Vec<CompNodeId>,
    /// Non-operand dependencies
    pub post: Vec<CompNodeId>,
}

impl CompNode {
    pub fn new(produces_value: bool, operands: Vec<CompNodeId>, post: Vec<CompNodeId>) -> Self {
        Self {
            blocked_by: Some(0),
            produces_value,
            operands,
            post,
        }
    }

    pub fn lone(produces_value: bool) -> Self {
        Self::new(produces_value, vec![], vec![])
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct IRGraph {
    pub input_ids: Vec<CompNodeId>,
    pub output_ids: Vec<CompNodeId>,
    pub nodes: Vec<CompNode>,
}
