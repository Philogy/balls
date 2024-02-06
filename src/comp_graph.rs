use crate::parser::types::Ident;
use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompResult {
    Op(Ident),
    Const(BigUint),
}

pub type CompNodeId = usize;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CompNode {
    id: CompNodeId,
    pub has_output: bool,
    operands: Vec<CompNodeId>,
    // Non-operand dependencies
    post: Vec<CompNodeId>,
    // Number of dependencies (operand + non-operand) preceding this node.
    pub blocked_by: usize,
}

impl CompNode {
    pub fn new(
        id: CompNodeId,
        has_output: bool,
        operands: Vec<CompNodeId>,
        post: Vec<CompNodeId>,
    ) -> Self {
        Self {
            id,
            has_output,
            operands,
            post,
            blocked_by: 0,
        }
    }
}
