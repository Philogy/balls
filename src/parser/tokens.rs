use crate::parser::types::Ident;

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
    // ============ Atoms =============
    Ident(Ident),
    Number(Vec<u8>),
    // =========== Symbols ============
    Arrow,
    OpenRound,
    CloseRound,
    OpenCurly,
    CloseCurly,
    OpenSquare,
    CloseSquare,
    Comma,
    Assign,
}
