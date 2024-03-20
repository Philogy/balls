use crate::parser::ast::{Ast, Expr, Function, HuffMacro, MacroArg};
use crate::transformer::std_evm::{get_standard_opcodes_and_deps, Op};

use crate::parser::types::Span;
use crate::parser::Spanned;
use std::collections::BTreeMap;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum SemanticError {
    DuplicateTopLevelIdentifier(String, Span, Span),
    HuffMacroMoreThanOneOut(Spanned<HuffMacro>),
    /// (duplicate_type, duplicate_instance)
    DuplicateIdentifier(String, Spanned<String>),
    AssigningToImmutableTopLevel(Span),
    /// (expected_identifier_type, referenced_identifier, span)
    UndeclaredIdentifier(String, Spanned<String>),
    /// (reference, uncallable_span)
    CallingNonCallable(Spanned<String>, Span),
    /// (expected, actual, argument_type, callable identifer, call span)
    CallArgumentMismatch(usize, usize, String, String, Span),
    /// (callable type, callable identifier, call span)
    NoOutputFromCall(String, String, Span),
    ReadAndWrite(Spanned<String>, Spanned<String>),
}

#[derive(Clone, Debug)]
pub enum Symbol {
    Dependency,
    Const,
    Op(Op),
    Function(Function),
    HuffMacro(HuffMacro),
}

pub type Symbols = BTreeMap<String, Spanned<Symbol>>;

fn validate_dependency_list(
    is_read: bool,
    symbols: &Symbols,
    dependencies: &Vec<Spanned<String>>,
) -> Vec<SemanticError> {
    dependencies
        .iter()
        .filter_map(|dep| match symbols.get(&dep.inner) {
            Some(Spanned {
                inner: Symbol::Dependency,
                ..
            }) => None,
            // TODO: Be more specific when identifier has different type than expected
            _ => Some(SemanticError::UndeclaredIdentifier(
                "dependency".into(),
                dep.clone(),
            )),
        })
        .chain(check_duplicate_identifiers(
            if is_read { "read" } else { "write" },
            &dependencies.iter().collect(),
        ))
        .collect()
}

fn validate_reads_writes(
    symbols: &Symbols,
    reads: &Vec<Spanned<String>>,
    writes: &Vec<Spanned<String>>,
) -> Vec<SemanticError> {
    let mut errors = vec![];
    errors.extend(validate_dependency_list(true, symbols, reads));
    errors.extend(validate_dependency_list(false, symbols, writes));

    if errors
        .iter()
        .all(|err| !matches!(err, SemanticError::DuplicateIdentifier(_, _)))
    {
        errors.extend(check_duplicate_identifiers(
            "reads-writes",
            &reads.iter().chain(writes.iter()).collect(),
        ));
    }

    errors
}

fn check_duplicate_identifiers(
    ident_type: &str,
    idents: &Vec<&Spanned<String>>,
) -> Vec<SemanticError> {
    idents
        .iter()
        .enumerate()
        .filter_map(|(i, ident)| {
            if ident.inner == "_" {
                return None;
            }
            for other_ident in &idents[(i + 1)..] {
                if ident.inner == *other_ident.inner {
                    return Some(SemanticError::DuplicateIdentifier(
                        ident_type.to_string(),
                        (*other_ident).clone(),
                    ));
                }
            }
            None
        })
        .collect()
}

fn validate_huff_macro(symbols: &Symbols, span: &Span, hmacro: &HuffMacro) -> Vec<SemanticError> {
    let mut errors = vec![];
    if hmacro.stack_out > 1 {
        errors.push(SemanticError::HuffMacroMoreThanOneOut(Spanned::new(
            hmacro.clone(),
            span.clone(),
        )));
    }

    errors.extend(check_duplicate_identifiers(
        "macro argument",
        &hmacro.macro_args.iter().collect(),
    ));

    errors.extend(validate_reads_writes(
        symbols,
        &hmacro.reads,
        &hmacro.writes,
    ));

    errors
}

fn validate_expression(
    func: &Function,
    top_level_symbols: &Symbols,
    local_symbols: &Vec<String>,
    expr: &Spanned<Expr>,
    expecting_output: bool,
    errors: &mut Vec<SemanticError>,
) {
    match &expr.inner {
        Expr::Call {
            ident,
            macro_args,
            stack_args,
        } => {
            let maybe_symbol = top_level_symbols.get(&ident.inner);
            if let Some(symbol) = maybe_symbol {
                match &symbol.inner {
                    Symbol::Dependency | Symbol::Const => {
                        errors.push(SemanticError::CallingNonCallable(
                            ident.clone(),
                            symbol.span.clone(),
                        ));
                    }
                    Symbol::Op(Op {
                        ident,
                        stack_in,
                        stack_out,
                        ..
                    }) => {
                        if *stack_in as usize != stack_args.inner.len() {
                            errors.push(SemanticError::CallArgumentMismatch(
                                *stack_in as usize,
                                stack_args.inner.len(),
                                "stack".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ));
                        }
                        if expecting_output && !stack_out {
                            errors.push(SemanticError::NoOutputFromCall(
                                "opcode".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ));
                        }
                        if !macro_args.inner.is_empty() {
                            errors.push(SemanticError::CallArgumentMismatch(
                                0,
                                macro_args.inner.len(),
                                "macro".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ))
                        }
                    }
                    Symbol::Function(Function {
                        ident,
                        macro_args: fn_macro_args,
                        inputs,
                        outputs,
                        ..
                    }) => {
                        if inputs.len() != stack_args.inner.len() {
                            errors.push(SemanticError::CallArgumentMismatch(
                                inputs.len(),
                                stack_args.inner.len(),
                                "stack".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ));
                        }
                        if fn_macro_args.len() != macro_args.inner.len() {
                            errors.push(SemanticError::CallArgumentMismatch(
                                fn_macro_args.len(),
                                macro_args.inner.len(),
                                "macro".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ))
                        }
                        if expecting_output && outputs.len() != 1 {
                            errors.push(SemanticError::NoOutputFromCall(
                                "function".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ));
                        }
                    }
                    Symbol::HuffMacro(HuffMacro {
                        ident,
                        macro_args: huff_macro_args,
                        stack_in,
                        stack_out,
                        ..
                    }) => {
                        if *stack_in as usize != stack_args.inner.len() {
                            errors.push(SemanticError::CallArgumentMismatch(
                                *stack_in as usize,
                                stack_args.inner.len(),
                                "stack".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ));
                        }
                        if huff_macro_args.len() != macro_args.inner.len() {
                            errors.push(SemanticError::CallArgumentMismatch(
                                huff_macro_args.len(),
                                macro_args.inner.len(),
                                "macro".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ))
                        }
                        if expecting_output && *stack_out != 1 {
                            errors.push(SemanticError::NoOutputFromCall(
                                "huff macro".into(),
                                ident.clone(),
                                expr.span.clone(),
                            ));
                        }
                    }
                }
            } else {
                errors.push(SemanticError::UndeclaredIdentifier(
                    "callable".into(),
                    ident.clone(),
                ));
            }
            for macro_arg in &macro_args.inner {
                match &macro_arg.inner {
                    MacroArg::Num(_) => {}
                    MacroArg::ArgRef(ident) => {
                        if !func
                            .macro_args
                            .iter()
                            .any(|macro_arg| &macro_arg.inner == ident)
                        {
                            errors.push(SemanticError::UndeclaredIdentifier(
                                "local macro arg, constant".into(),
                                Spanned::new(ident.clone(), macro_arg.span.clone()),
                            ));
                        }
                    }
                }
            }
            for stack_arg in stack_args.inner.iter() {
                validate_expression(
                    func,
                    top_level_symbols,
                    local_symbols,
                    stack_arg,
                    true,
                    errors,
                );
            }
        }
        Expr::Var(ident) => {
            if !func
                .macro_args
                .iter()
                .any(|macro_arg| macro_arg.inner == *ident)
                && !local_symbols.contains(ident)
                && !matches!(
                    top_level_symbols.get(ident),
                    Some(Spanned {
                        inner: Symbol::Const,
                        ..
                    })
                )
            {
                errors.push(SemanticError::UndeclaredIdentifier(
                    "local variable, top-level constant".into(),
                    Spanned::new(ident.clone(), expr.span.clone()),
                ))
            }
        }
        Expr::Num(_) => {} // Nothing to validate (size is validated in the parser)
    }
}

fn validate_func(symbols: &Symbols, func: &Function) -> Vec<SemanticError> {
    let mut errors = vec![];

    let func_args: Vec<_> = func.macro_args.iter().chain(func.inputs.iter()).collect();

    errors.extend(check_duplicate_identifiers("function argument", &func_args));

    let mut local_symbols: Vec<String> = func
        .inputs
        .iter()
        .map(|input| input.inner.clone())
        .collect();

    for statement in func.body.iter() {
        validate_expression(
            func,
            symbols,
            &local_symbols,
            &statement.expr,
            statement.ident.is_some(),
            &mut errors,
        );
        if let Some(spanned_ident) = &statement.ident {
            if func
                .macro_args
                .iter()
                .any(|macro_arg| macro_arg.inner == spanned_ident.inner)
                || symbols.get(&spanned_ident.inner).is_some()
            {
                errors.push(SemanticError::AssigningToImmutableTopLevel(
                    spanned_ident.span.clone(),
                ));
            }
            if !local_symbols.contains(&spanned_ident.inner) {
                local_symbols.push(spanned_ident.inner.clone());
            }
        }
    }

    for output in &func.outputs {
        if !local_symbols.contains(&output.inner) {
            errors.push(SemanticError::UndeclaredIdentifier(
                "local variable".into(),
                output.clone(),
            ));
        }
    }

    errors.extend(validate_reads_writes(symbols, &func.reads, &func.writes));

    errors
}

pub fn validate_and_get_symbols(nodes: Vec<Spanned<Ast>>) -> Result<Symbols, Vec<SemanticError>> {
    let mut symbols = Symbols::new();

    let (std_deps, std_ops) = get_standard_opcodes_and_deps();
    for dep in std_deps {
        if symbols
            .insert(dep.into(), Spanned::new(Symbol::Dependency, 0..0))
            .is_some()
        {
            panic!("Duplicate symbol from std_lib")
        }
    }
    for op in &std_ops {
        if symbols
            .insert(op.ident.clone(), Spanned::new(Symbol::Op(op.clone()), 0..0))
            .is_some()
        {
            panic!("Duplicate symbol from std_lib")
        }
    }
    for op in std_ops {
        if let Some((other_ident, _)) = &op.other {
            match symbols.get(other_ident) {
                Some(Spanned {
                    inner: Symbol::Op(other_op),
                    ..
                }) => {
                    assert!(
                        op.stack_in == other_op.stack_in && op.stack_out == other_op.stack_out,
                        "Mismatching variant op in std_ops {:?} vs. {:?}",
                        op,
                        other_op
                    );
                    assert!(
                        op.stack_out,
                        "std_ops marked non-outputing opcode as having variant {:?}",
                        op
                    );
                }
                _ => panic!(
                    "Variant op from std_ops not found or is not an op symbol: {}",
                    other_ident
                ),
            }
        }
    }

    let mut errors: Vec<SemanticError> = nodes
        .into_iter()
        .filter_map(|Spanned { inner: node, span }| {
            let (identifier, symbol) = match node {
                Ast::Const(ident) => Some((ident, Symbol::Const)),
                Ast::Dependency(ident) => Some((ident, Symbol::Dependency)),
                Ast::Function(func) => Some((func.ident.clone(), Symbol::Function(func))),
                Ast::HuffMacro(hmacro) => Some((hmacro.ident.clone(), Symbol::HuffMacro(hmacro))),
                Ast::Error => None,
            }?;
            let duplicate_node =
                symbols.insert(identifier.clone(), Spanned::new(symbol, span.clone()))?;
            Some(SemanticError::DuplicateTopLevelIdentifier(
                identifier,
                duplicate_node.span,
                span,
            ))
        })
        .collect();
    errors.extend(symbols.values().flat_map(|symbol| {
        match &symbol.inner {
            Symbol::Function(func) => validate_func(&symbols, func),
            Symbol::HuffMacro(hmacro) => validate_huff_macro(&symbols, &symbol.span, hmacro),
            Symbol::Op(_) | Symbol::Const | Symbol::Dependency => vec![], // Nothing to validate, no errors
        }
    }));

    if errors.is_empty() {
        Ok(symbols)
    } else {
        Err(errors)
    }
}
