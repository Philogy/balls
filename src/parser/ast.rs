use std::boxed::Box;

use crate::parser::types::{Ident, Spanned};
use num_bigint::BigUint;

#[derive(Clone, Debug)]
pub enum Expr {
    Call {
        name: Spanned<Ident>,
        args: Spanned<Box<Vec<Spanned<Expr>>>>,
    },
    Var(Ident),
    Num(BigUint),
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub ident: Option<Spanned<Ident>>,
    pub expr: Spanned<Expr>,
}

#[derive(Clone, Debug)]
pub struct OpDefinition {
    pub name: Ident,
    pub stack_in: u16,
    pub stack_out: u16,
    pub reads: Vec<Ident>,
    pub writes: Vec<Ident>,
}

#[derive(Clone, Debug)]
pub struct Macro {
    pub name: Ident,
    pub top_level_reads: Vec<Spanned<Ident>>,
    pub inputs: Vec<Ident>,
    pub outputs: Vec<Ident>,
    pub body: Vec<Statement>,
}

#[derive(Clone, Debug)]
pub enum Ast {
    Macro(Macro),
    OpDef(OpDefinition),
    Dependency(Ident),
    Error,
}
