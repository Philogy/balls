// The computational graph can be considered the "IR" of balls.

use crate::parser::ast::{Expr, Function, HuffMacro, MacroArg};
use crate::parser::Spanned;
use crate::scheduling::ir::{CompNode, CompNodeId, IRGraph};
use crate::transformer::analysis::{Symbol, Symbols};
use crate::transformer::std_evm::Op;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub enum ValueSource {
    TopLevelInput(String),
    Op(String),
    MacroInvoke(String, Vec<MacroArg>),
    MacroArg(MacroArg),
    HuffConst(String),
}

impl ValueSource {
    pub fn huff_repr(&self, symbols: &Symbols, using_variant: bool) -> String {
        match self {
            Self::Op(ident) => {
                if !using_variant {
                    ident.clone()
                } else {
                    match symbols.get(ident).expect("Invalid source identifier") {
                        Spanned {
                            inner: Symbol::Op(op),
                            ..
                        } => op
                            .other
                            .as_ref()
                            .expect("Using variant flag with non-variant op")
                            .0
                            .clone(),
                        unexpected => panic!("Expected op symbol, not {:?}", unexpected),
                    }
                }
            }
            Self::TopLevelInput(ident) => ident.clone(),
            Self::MacroInvoke(ident, args) => format!(
                "{}({})",
                ident,
                args.iter()
                    .map(MacroArg::huff_repr)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),

            Self::MacroArg(arg) => arg.huff_repr(),
            Self::HuffConst(ident) => format!("[{}]", ident),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SemanticContext {
    top_level_macro_args: Vec<String>,
    pub nodes_sources: Vec<(CompNode, ValueSource)>,
    ident_to_id: HashMap<String, CompNodeId>,
    last_write: HashMap<String, CompNodeId>,
    last_reads: HashMap<String, Vec<CompNodeId>>,
}

impl SemanticContext {
    pub fn new(top_level_macro_args: Vec<String>) -> Self {
        Self {
            top_level_macro_args,
            ..Default::default()
        }
    }
    pub fn add_node(&mut self, node: CompNode, value_source: ValueSource) -> CompNodeId {
        let id = self.nodes_sources.len();
        self.nodes_sources.push((node, value_source));
        id
    }

    pub fn set_ident(&mut self, ident: String, id: CompNodeId) {
        self.ident_to_id.insert(ident, id);
    }

    pub fn get_with_symbols(&mut self, symbols: &Symbols, ident: &String) -> Option<CompNodeId> {
        if let Some(id) = self.get_ident(ident) {
            return Some(id);
        }
        if let Some(Spanned {
            inner: Symbol::Const,
            ..
        }) = symbols.get(ident)
        {
            return Some(
                self.add_node(CompNode::lone(true), ValueSource::HuffConst(ident.clone())),
            );
        }
        None
    }

    pub fn get_ident(&mut self, ident: &String) -> Option<CompNodeId> {
        if let Some(id) = self.ident_to_id.get(ident) {
            return Some(*id);
        }
        if self.top_level_macro_args.contains(ident) {
            let id = self.add_node(
                CompNode::lone(true),
                ValueSource::MacroArg(MacroArg::ArgRef(ident.clone())),
            );
            return Some(id);
        }
        None
    }

    pub fn get_last_write(&self, ident: &String) -> Option<CompNodeId> {
        let last = self.last_write.get(ident)?;
        Some(*last)
    }

    pub fn record_read(&mut self, dependency: &String, id: CompNodeId) {
        let reading = self.last_reads.entry(dependency.clone()).or_default();
        reading.push(id);

        if let Some(last_write_id) = self.last_write.get(dependency).copied() {
            self.nodes_sources[id].0.post.push(last_write_id);
        }
    }

    /// Returns the IDs of the nodes that the newly inserted node is now dependent on (non-operand
    /// semantic dependency).
    pub fn record_write(&mut self, dependency: &String, id: CompNodeId) {
        let mut pre_deps = self
            .last_reads
            .insert(dependency.clone(), Vec::new())
            .unwrap_or_default();
        pre_deps.extend(self.last_write.insert(dependency.clone(), id));

        self.nodes_sources[id].0.post.extend(pre_deps);
    }
}

fn unspan<T: Clone + Debug>(spanned: &Vec<Spanned<T>>) -> Vec<T> {
    spanned.iter().map(Spanned::unwrap_ref).cloned().collect()
}

/// Graphs an expression object, transforming and creating nodes
fn graph_expr(ctx: &mut SemanticContext, symbols: &Symbols, expr: &Expr) -> CompNodeId {
    match expr {
        Expr::Var(ident) => ctx.get_with_symbols(symbols, &ident).unwrap_or_else(|| {
            panic!(
                "Encountered invalid identifier in IR gen ({}, {:?})",
                ident, ctx.top_level_macro_args
            )
        }),
        Expr::Num(num) => ctx.add_node(
            CompNode::lone(true),
            ValueSource::MacroArg(MacroArg::Num(num.clone())),
        ),
        Expr::Call {
            ident,
            macro_args,
            stack_args,
        } => {
            let arg_ids: Vec<_> = stack_args
                .inner
                .iter()
                .map(|e| graph_expr(ctx, symbols, &e.inner))
                .collect();
            let symbol = symbols
                .get(&ident.inner)
                .expect("Encountered invalid identifier in IR gen");

            let (value_source, reads, writes, produces_value): (
                ValueSource,
                Vec<String>,
                Vec<String>,
                bool,
            ) = match &symbol.inner {
                Symbol::Function(Function {
                    ident,
                    reads,
                    writes,
                    outputs,
                    ..
                }) => (
                    ValueSource::MacroInvoke(ident.clone(), unspan(&macro_args.inner)),
                    unspan(reads),
                    unspan(writes),
                    outputs.len() == 1,
                ),
                Symbol::HuffMacro(HuffMacro {
                    ident,
                    reads,
                    writes,
                    stack_out,
                    ..
                }) => (
                    ValueSource::MacroInvoke(ident.clone(), unspan(&macro_args.inner)),
                    unspan(reads),
                    unspan(writes),
                    *stack_out == 1,
                ),
                Symbol::Op(Op {
                    ident,
                    reads,
                    writes,
                    stack_out,
                    ..
                }) => (
                    ValueSource::Op(ident.clone()),
                    reads.clone(),
                    writes.clone(),
                    *stack_out,
                ),
                other => panic!("Uncallable symbol in IR gen {:?}", other),
            };

            for arg_id in arg_ids.iter() {
                debug_assert!(ctx.nodes_sources[*arg_id].0.produces_value);
            }

            let id = ctx.add_node(CompNode::new(produces_value, arg_ids, vec![]), value_source);

            for r in reads.iter() {
                ctx.record_read(r, id);
            }
            for w in writes.iter() {
                ctx.record_write(w, id);
            }

            id
        }
    }
}

fn set_blocked_count(input_ids: &[CompNodeId], output_ids: &[CompNodeId], nodes: &mut [CompNode]) {
    let total = nodes.len();

    let mut blocked_by = vec![0u32; total];
    let mut stack_count = vec![0u32; total];

    for node in nodes.iter() {
        for post_id in node.post.iter() {
            blocked_by[*post_id] += 1;
        }
        for dep_id in node.operands.iter() {
            blocked_by[*dep_id] += 1;
            // Blocked once as an argument.
            stack_count[*dep_id] += 1;
        }
    }

    for output_id in output_ids.iter() {
        stack_count[*output_id] += 1;
    }

    for id in 0..total {
        let required_dedups = stack_count[id].max(1) - 1;
        *nodes[id].blocked_by.as_mut().unwrap() += required_dedups + blocked_by[id];
    }

    for id in 0..total {
        if nodes[id].blocked_by.unwrap() == 0 && input_ids.contains(&id) && output_ids.contains(&id)
        {
            nodes[id].blocked_by = None;
        }
    }
}

// Needs to return
pub fn gen_ir(
    func: &Function,
    symbols: &Symbols,
) -> (IRGraph, Vec<ValueSource>, Vec<(String, CompNodeId)>) {
    // Assign IDs to inputs and validate uniqueness.
    let mut ctx = SemanticContext::new(
        func.macro_args
            .iter()
            .map(|spanned| spanned.inner.clone())
            .collect(),
    );

    // Verify that there are no duplicate input identifiers and create nodes.
    let input_ids: Vec<_> = func
        .inputs
        .iter()
        .map(|Spanned { inner: ident, .. }| {
            let id = ctx.add_node(
                CompNode::lone(true),
                ValueSource::TopLevelInput(ident.clone()),
            );
            ctx.set_ident(ident.clone(), id);
            id
        })
        .collect();

    // Assign IDs to statements.
    let assignments: Vec<_> = func
        .body
        .iter()
        .filter_map(|statement| {
            // Convert nested expressions to nodes and assign IDs
            let id = graph_expr(&mut ctx, symbols, &statement.expr.inner);

            let spanned_ident = statement.ident.as_ref()?;
            let ident = spanned_ident.inner.clone();

            ctx.set_ident(ident.clone(), id);

            Some((ident, id))
        })
        .collect();

    // Validate outputs and retrieve their IDs.
    let output_ids: Vec<_> = func
        .outputs
        .iter()
        .map(|output| {
            ctx.get_with_symbols(symbols, &output.inner)
                .expect(format!("TODO: Undefined output identifer {:?}", output).as_str())
        })
        .collect();

    let (mut nodes, sources): (Vec<CompNode>, Vec<ValueSource>) =
        ctx.nodes_sources.into_iter().unzip();

    set_blocked_count(
        input_ids.as_slice(),
        output_ids.as_slice(),
        nodes.as_mut_slice(),
    );

    let variants: Vec<Option<Vec<usize>>> = sources
        .iter()
        .map(|src| {
            let symbol = match src {
                ValueSource::Op(ident) => {
                    Some(&symbols.get(ident).expect("Invalid op ident").inner)
                }
                _ => None,
            }?;
            let other = match symbol {
                Symbol::Op(op) => op.other.as_ref(),
                _ => None,
            }?;
            Some(other.1.clone())
        })
        .collect();

    (
        IRGraph {
            input_ids,
            output_ids,
            nodes,
            variants,
        },
        sources,
        assignments,
    )
}
