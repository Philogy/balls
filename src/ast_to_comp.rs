use crate::comp_graph::{CompNode, CompNodeId, Computation};
use crate::parser::ast::{Ast, Expr, Macro, OpDefinition};
use crate::parser::types::{Ident, Spanned};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

pub type SymbolId = usize;

pub fn sort_ast_nodes(
    nodes: Vec<Spanned<Ast>>,
) -> (
    Vec<Spanned<Ident>>,
    Vec<Spanned<OpDefinition>>,
    Vec<Spanned<Macro>>,
) {
    let mut dependencies = Vec::new();
    let mut ops = Vec::new();
    let mut macros = Vec::new();

    for Spanned { inner: node, span } in nodes {
        match node {
            Ast::Dependency(ident) => dependencies.push(Spanned::new(ident, span)),
            Ast::OpDef(op) => ops.push(Spanned::new(op, span)),
            Ast::Macro(r#macro) => macros.push(Spanned::new(r#macro, span)),
            Ast::Error => {}
        }
    }

    (dependencies, ops, macros)
}

#[derive(Debug)]
pub struct UniqueSet<T: Hash + Debug + Eq>(HashSet<T>);

impl<T: Hash + Debug + Eq> UniqueSet<T> {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn contains(&self, value: &T) -> bool {
        self.0.contains(value)
    }

    pub fn add<'a>(&mut self, value: T, group: &'a str) {
        assert!(
            !self.contains(&value),
            "TODO: Duplicate value {:?} in {}.",
            &value,
            group
        );
        self.0.insert(value);
    }
}

/// TODO: Actual error handling
pub fn validate_and_extract_globals(
    dependencies: Vec<Spanned<Ident>>,
    ops: Vec<Spanned<OpDefinition>>,
    macros: Vec<Spanned<Macro>>,
) -> (Vec<Ident>, Vec<OpDefinition>, Vec<Macro>) {
    let mut unique_globals = UniqueSet::new();

    let mut unique_dependencies = UniqueSet::new();

    let dependencies = dependencies
        .into_iter()
        .map(|Spanned { inner, .. }| {
            unique_globals.add(inner.clone(), "globals");
            unique_dependencies.add(inner.clone(), "dependencies");
            inner
        })
        .collect();

    let mut unique_ops = UniqueSet::new();
    let ops = ops
        .into_iter()
        .map(|Spanned { inner, .. }| {
            unique_globals.add(inner.name.clone(), "globals");
            unique_ops.add(inner.name.clone(), "ops");
            assert!(
                inner.stack_out <= 1,
                "TODO: More than two stack out not currently supported ({})",
                inner.name.clone()
            );
            inner.writes.iter().for_each(|w| {
                assert!(
                    unique_dependencies.0.iter().any(|dep| dep == w),
                    "TODO: Nonexistant write dependency {:?}",
                    w
                );
            });
            inner.reads.iter().for_each(|r| {
                assert!(
                    unique_dependencies.0.iter().any(|dep| dep == r),
                    "TODO: Nonexistant read dependency {:?}",
                    r
                );
                assert!(
                    inner.writes.iter().all(|w| r != w),
                    "TODO: Ops that read & write to the same dependency is not yet supported"
                );
            });
            inner
        })
        .collect();

    let mut unique_macros = UniqueSet::new();
    let macros = macros
        .into_iter()
        .map(|Spanned { inner, .. }| {
            unique_globals.add(inner.name.clone(), "globals");
            unique_macros.add(inner.name.clone(), "macros");
            inner
        })
        .collect();

    (dependencies, ops, macros)
}

const RESERVED_EMPTY_IDENTIFIER: &str = "_";

struct SemanticContext<'a> {
    ops: &'a Vec<OpDefinition>,
    next_id: CompNodeId,
    nodes: Vec<(CompNode, Computation)>,
    ident_to_id: HashMap<Ident, CompNodeId>,
    last_write: HashMap<Ident, CompNodeId>,
    last_reads: HashMap<Ident, Vec<CompNodeId>>,
}

impl<'a> SemanticContext<'a> {
    pub fn new(ops: &'a Vec<OpDefinition>) -> Self {
        Self {
            ops,
            next_id: 0,
            nodes: Vec::new(),
            ident_to_id: HashMap::new(),
            last_write: HashMap::new(),
            last_reads: HashMap::new(),
        }
    }

    pub fn new_id(&mut self) -> CompNodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn set_ident(&mut self, ident: Ident, id: CompNodeId) {
        if ident != RESERVED_EMPTY_IDENTIFIER {
            self.ident_to_id.insert(ident, id);
        }
    }

    pub fn get_ident(&self, ident: &Ident) -> Option<&CompNodeId> {
        self.ident_to_id.get(ident)
    }

    pub fn get_has_output(&self, id: CompNodeId) -> Result<bool, String> {
        match self.nodes.get(id) {
            Some((node, _)) => Ok(node.has_output),
            None => Err(format!("Invalid comp id {}", id)),
        }
    }

    pub fn get_op(&self, ident: &Ident) -> Option<&'a OpDefinition> {
        self.ops.iter().filter(|op| &op.name == ident).next()
    }

    /// Expect valid id.
    fn inc_blocked_by_count(&mut self, id: &CompNodeId) {
        let (node, _) = self
            .nodes
            .get_mut(*id)
            .expect(format!("Expected valid comp id, got: {}", id).as_str());
        node.blocked_by += 1;
    }

    pub fn record_read(&mut self, dependency: &Ident, id: CompNodeId) -> Option<CompNodeId> {
        let reading = self.last_reads.entry(dependency.clone()).or_default();
        reading.push(id);

        let write_id = *self.last_write.get(dependency)?;

        self.inc_blocked_by_count(&write_id);

        Some(write_id)
    }

    pub fn record_write(&mut self, dependency: &Ident, id: CompNodeId) -> Vec<CompNodeId> {
        let mut prev_reads = self
            .last_reads
            .insert(dependency.clone(), Vec::new())
            .unwrap_or_default();

        prev_reads.extend(self.last_write.insert(dependency.clone(), id));

        prev_reads
            .iter()
            .for_each(|prev_read_id| self.inc_blocked_by_count(prev_read_id));

        prev_reads
    }
}

fn map_expr(ctx: &mut SemanticContext, expr: &Expr) -> (usize, bool) {
    match expr {
        Expr::Call { name, args } => {
            let mapped_args: Vec<_> = args
                .inner
                .iter()
                .rev()
                .map(|e| map_expr(ctx, &e.inner))
                .collect();

            let ident = &name.inner;
            let op = ctx
                .get_op(ident)
                .expect(format!("TODO: Invalid op {:?} referenced", ident).as_str());

            assert_eq!(
                mapped_args.len(),
                op.stack_in as usize,
                "TODO: Expected {} argument(s) received: {}",
                mapped_args.len(),
                op.stack_in
            );

            mapped_args
                .iter()
                .enumerate()
                .for_each(|(i, (_, has_output))| {
                    assert!(
                        has_output,
                        "TODO: Argument #{} does not return an output",
                        i + 1
                    );
                });

            let id = ctx.new_id();
            let mut post_ids: Vec<CompNodeId> = Vec::new();
            for r in op.reads.iter() {
                post_ids.extend(ctx.record_read(r, id));
            }
            for w in op.writes.iter() {
                post_ids.extend(ctx.record_write(w, id));
            }

            let has_output = op.stack_out == 1;

            let arg_ids = mapped_args
                .into_iter()
                .map(|(id, _)| {
                    ctx.inc_blocked_by_count(&id);
                    id
                })
                .collect();

            ctx.nodes.push((
                CompNode::new(id, has_output, arg_ids, post_ids),
                Computation::Op(op.name.clone()),
            ));

            (id, has_output)
        }
        Expr::Var(ident) => {
            let id = *ctx
                .get_ident(&ident)
                .expect(format!("TODO: Variable {:?} not yet defined", &ident).as_str());
            let has_output = ctx
                .get_has_output(id)
                .expect("get_ident returned invalid id");
            (id, has_output)
        }
        Expr::Num(num) => {
            let id = ctx.new_id();
            ctx.nodes.push((
                CompNode::new(id, true, vec![], vec![]),
                Computation::Const(num.clone()),
            ));
            (id, true)
        }
    }
}

pub struct TransformedMacro {
    pub nodes: Vec<(CompNode, Computation)>,
    pub output_ids: Vec<CompNodeId>,
    pub statement_ids: Vec<CompNodeId>,
}

pub fn transform_macro(
    dependencies: &Vec<Ident>,
    ops: &Vec<OpDefinition>,
    macro_def: Macro,
) -> TransformedMacro {
    // Assign IDs to inputs and validate uniqueness.
    let mut ctx = SemanticContext::new(ops);

    // Verify that there are no duplicate input identifiers and create nodes.
    macro_def
        .inputs
        .iter()
        .enumerate()
        .for_each(|(i, input_ident)| {
            assert!(
                !macro_def.inputs[(i + 1)..].contains(input_ident),
                "TODO: Duplicate input identifier {}",
                input_ident
            );

            let id = ctx.new_id();

            ctx.set_ident(input_ident.clone(), id);

            ctx.nodes.push((
                CompNode::new(id, true, vec![], vec![]),
                Computation::TopLevelInput(input_ident.clone()),
            ));
        });

    // Assign IDs to statements.
    let statement_ids: Vec<_> = macro_def
        .body
        .iter()
        .map(|statement| {
            // Convert nested expressions to nodes and assign IDs
            let (id, has_output) = map_expr(&mut ctx, &statement.expr.inner);
            assert_eq!(
                has_output,
                statement.ident.is_some(),
                "TODO: The number of operation outputs must be equal to the variable assignments"
            );
            if let Some(ident) = &statement.ident {
                let ident = ident.inner.clone();
                ctx.set_ident(ident, id);
            }

            id
        })
        .collect();

    // Validate outputs and retrieve their IDs.
    let output_ids: Vec<_> = macro_def
        .outputs
        .iter()
        .map(|output| {
            *ctx.get_ident(output)
                .expect(format!("TODO: Undefined ouput identifer {:?}", output).as_str())
        })
        .collect();

    let top_level_dependencies: Vec<CompNodeId> = macro_def
        .top_level_reads
        .into_iter()
        .filter_map(|Spanned { inner: ident, .. }| {
            dependencies
                .iter()
                .find(|dependency| **dependency == ident)
                .expect(format!("TODO: Referencing nonexistent dependency {:?}", ident).as_str());
            Some(*ctx.last_write.get(&ident)?)
        })
        .collect();

    for (id, statement) in statement_ids.iter().zip(macro_def.body) {
        // Ids in statement_ids are expected to be valid.
        let (node, res) = ctx.nodes.get(*id).unwrap();

        let count = node.blocked_by;
        if count == 0
            && top_level_dependencies.iter().all(|dep_id| dep_id != id)
            && output_ids.iter().all(|dep_id| dep_id != id)
        {
            println!(
                "TODO_WARNING: {:?} = {:?} unused. Only top-level inputs can remain unused. ({}) count: {}",
                statement.ident,
                res,
                id,
                count
            );
        }
    }

    TransformedMacro {
        nodes: ctx.nodes,
        output_ids,
        statement_ids,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        let macro_def = Macro {
            name: "empty".into(),
            top_level_reads: vec![],
            inputs: vec![],
            outputs: vec![],
            body: vec![],
        };

        let transform = transform_macro(&vec![], &vec![], macro_def);

        assert_eq!(transform.nodes, vec![]);
        assert_eq!(transform.output_ids, vec![]);
        assert_eq!(transform.statement_ids, vec![]);
    }
}
