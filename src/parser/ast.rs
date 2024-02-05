use std::boxed::Box;

use crate::parser::types::Ident;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    Call { name: Ident, args: Box<Vec<Expr>> },
    Var(Ident),
    Num(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    pub ident: Option<Ident>,
    pub expr: Expr,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpDefinition {
    pub name: Ident,
    pub stack_in: u16,
    pub stack_out: u16,
    pub reads: Vec<Ident>,
    pub writes: Vec<Ident>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Macro {
    pub name: Ident,
    pub inputs: Vec<Ident>,
    pub outputs: Vec<Ident>,
    pub body: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ast {
    Macro(Macro),
    OpDef(OpDefinition),
    Dependency(Ident),
}
