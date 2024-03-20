use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    // ====== Top-level Keywords ======
    Fn,
    Op,
    Dependency,
    External,
    Const,
    // ========= Sub Keywords =========
    Stack,
    Reads,
    Writes,
    // ============ Atoms =============
    Ident(String),
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

impl From<Token> for String {
    fn from(val: Token) -> Self {
        format!("{}", val)
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
