use std::boxed::Box;

use crate::parser::types::Spanned;
use num_bigint::BigUint;

#[derive(Clone, Debug)]
pub enum MacroArg {
    ArgRef(String),
    Num(BigUint),
}

impl MacroArg {
    pub fn huff_repr(&self) -> String {
        match self {
            Self::Num(num) => format!("0x{:x}", num),
            Self::ArgRef(ident) => format!("<{}>", ident),
        }
    }

    pub fn balls_repr(&self) -> String {
        match self {
            Self::Num(num) => format!("0x{:x}", num),
            Self::ArgRef(ident) => format!("{}", ident),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Expr {
    Call {
        ident: Spanned<String>,
        macro_args: Spanned<Vec<Spanned<MacroArg>>>,
        stack_args: Spanned<Box<Vec<Spanned<Expr>>>>,
    },
    Var(String),
    Num(BigUint),
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub ident: Option<Spanned<String>>,
    pub expr: Spanned<Expr>,
}

#[derive(Clone, Debug)]
pub struct HuffMacro {
    pub ident: String,
    pub macro_args: Vec<Spanned<String>>,
    pub stack_in: u16,
    pub stack_out: u16,
    pub reads: Vec<Spanned<String>>,
    pub writes: Vec<Spanned<String>>,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub ident: String,
    pub macro_args: Vec<Spanned<String>>,
    pub inputs: Vec<Spanned<String>>,
    pub outputs: Vec<Spanned<String>>,
    pub body: Vec<Statement>,
    pub reads: Vec<Spanned<String>>,
    pub writes: Vec<Spanned<String>>,
}

#[derive(Clone, Debug)]
pub enum Ast {
    Dependency(String),
    Const(String),
    Function(Function),
    HuffMacro(HuffMacro),
    Error,
}
