use crate::parser::types::Ident;
use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Computation {
    Op(Ident),
    Const(BigUint),
    TopLevelInput(Ident),
}

pub type CompNodeId = usize;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CompNode {
    pub id: CompNodeId,
    pub has_output: bool,
    pub operands: Vec<CompNodeId>,
    /// Non-operand dependencies
    pub post: Vec<CompNodeId>,
    /// Number of dependencies (operand + non-operand) preceding this node.
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
