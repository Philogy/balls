use crate::parser::types::Ident;
use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    // ============= Core =============
    Define,
    // ====== Top-level Keywords ======
    Op,
    Dependency,
    Macro,
    // ========= Sub Keywords =========
    Stack,
    Reads,
    Writes,
    External,
    // ============ Atoms =============
    Ident(Ident),
    Number(BigUint),
    // =========== Symbols ============
    Arrow,
    OpenRound,
    CloseRound,
    OpenCurly,
    CloseCurly,
    OpenSquare,
    CloseSquare,
    OpenAngle,
    CloseAngle,
    Comma,
    Assign,
}

impl Into<String> for Token {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(num) => write!(f, "Number({:x})", num),
            token => write!(f, "{:?}", token),
        }
    }
}
