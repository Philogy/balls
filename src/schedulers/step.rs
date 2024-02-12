use crate::comp_graph::CompNodeId;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Ord, PartialOrd)]
pub enum Step {
    Swap(usize),
    Dup(usize),
    Pop,
    Op(CompNodeId),
}

impl Step {
    pub fn cost(&self) -> u32 {
        match self {
            Self::Swap(_) => 1,
            Self::Dup(_) => 1,
            Self::Pop => 1,
            Self::Op(_) => 0,
        }
    }
}
