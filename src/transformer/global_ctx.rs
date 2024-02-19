use crate::comp_graph::{CompNode, CompNodeId, Computation};
use crate::parser::ast::{Ast, Expr, Macro, OpDefinition};
use crate::parser::{Ident, Spanned};
use crate::transformer::semantics::SemanticContext;
use crate::transformer::TransformedMacro;
use crate::Searchable;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

fn sort_ast_nodes(
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

impl<T: Clone + Hash + Debug + Eq> UniqueSet<T> {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn add<'a>(&mut self, value: T, group: &'a str) {
        assert!(
            self.0.insert(value.clone()),
            "TODO: Duplicate value {:?} in {}.",
            &value,
            group
        );
    }
}

fn validate_and_extract_globals(
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
                    unique_dependencies.0.iter().contains(w),
                    "TODO: Nonexistant write dependency {:?}",
                    w
                );
            });
            inner.reads.iter().for_each(|r| {
                assert!(
                    unique_dependencies.0.iter().contains(r),
                    "TODO: Nonexistant read dependency {:?}",
                    r
                );
                assert!(
                    !inner.writes.iter().contains(r),
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

#[derive(Debug, Clone, Default)]
pub struct GlobalContext {
    pub deps: Vec<Ident>,
    pub ops: Vec<OpDefinition>,
    pub macros: Vec<Macro>,
}

impl GlobalContext {
    pub fn transform(&self, macro_def: Macro) -> TransformedMacro {
        // Assign IDs to inputs and validate uniqueness.
        let mut ctx = SemanticContext::default();

        // Verify that there are no duplicate input identifiers and create nodes.
        let input_ids = macro_def
            .inputs
            .iter()
            .enumerate()
            .map(|(i, input_ident)| {
                assert!(
                    !macro_def.inputs[(i + 1)..].contains(input_ident),
                    "TODO: Duplicate input identifier {}",
                    input_ident
                );

                let id = ctx.new_id();

                ctx.set_ident(input_ident.clone(), id);

                ctx.nodes.push((
                    CompNode::lone(id, true),
                    Computation::TopLevelInput(input_ident.clone()),
                ));

                id
            })
            .collect();

        // Assign IDs to statements.
        let assignments: Vec<_> = macro_def
            .body
            .iter()
            .filter_map(|statement| {
                // Convert nested expressions to nodes and assign IDs
                let (id, has_output) = self.map_expr(&mut ctx, &statement.expr.inner);
                assert_eq!(
                    has_output,
                    statement.ident.is_some(),
                    "TODO: The number of operation outputs must be equal to the variable assignments ({:?} =)",
                    statement.ident
                );

                let spanned_ident = statement.ident.as_ref()?;
                let ident = spanned_ident.inner.clone();

                ctx.set_ident(ident.clone(), id);
                // TODO: Prevent more kinds of shadowing.
                self.ops.iter().for_each(|op| {
                    assert!(&ident != &op.name, "TODO: Variable named {:?} shadows existing op definition", &ident)
                });

                Some((ident, id))
            })
            .collect();

        // Validate outputs and retrieve their IDs.
        let output_ids: Vec<_> = macro_def
            .outputs
            .iter()
            .map(|output| {
                *ctx.get_ident(output)
                    .expect(format!("TODO: Undefined output identifer {:?}", output).as_str())
            })
            .collect();

        let top_level_deps: Vec<CompNodeId> = macro_def
            .top_level_reads
            .into_iter()
            .filter_map(|Spanned { inner: ident, .. }| {
                if !self.deps.iter().contains(&ident) {
                    panic!("TODO: Referencing nonexistent dependency {:?}", ident);
                }
                ctx.get_last_write(&ident)
            })
            .collect();

        TransformedMacro {
            nodes: ctx.nodes,
            input_ids,
            output_ids,
            assignments,
            top_level_deps,
        }
    }

    fn map_expr(&self, ctx: &mut SemanticContext, expr: &Expr) -> (usize, bool) {
        match expr {
            Expr::Call { name, args } => {
                let mapped_args: Vec<_> = args
                    .inner
                    .iter()
                    .map(|e| self.map_expr(ctx, &e.inner))
                    .collect();

                let ident = &name.inner;
                let op = self
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

                let arg_ids = mapped_args.into_iter().map(|(id, _)| id).collect();

                let comp = if op.external {
                    Computation::External
                } else {
                    Computation::Op
                };

                ctx.nodes.push((
                    CompNode::new(id, has_output, arg_ids, post_ids),
                    comp(op.name.clone()),
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
                ctx.nodes
                    .push((CompNode::lone(id, true), Computation::Const(num.clone())));
                (id, true)
            }
        }
    }

    pub fn get_op(&self, ident: &Ident) -> Option<&OpDefinition> {
        self.ops.iter().filter(|op| &op.name == ident).next()
    }
}

impl From<Vec<Spanned<Ast>>> for GlobalContext {
    fn from(ast_nodes: Vec<Spanned<Ast>>) -> Self {
        let (deps, ops, macros) = sort_ast_nodes(ast_nodes);
        let (deps, ops, macros) = validate_and_extract_globals(deps, ops, macros);

        Self { deps, ops, macros }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        let ctx = GlobalContext::default();

        let macro_def = Macro {
            name: "empty".into(),
            top_level_reads: vec![],
            inputs: vec![],
            outputs: vec![],
            body: vec![],
        };

        let transform = ctx.transform(macro_def);

        assert_eq!(transform.nodes, vec![]);
        assert_eq!(transform.output_ids, vec![]);
        assert_eq!(transform.assignments, vec![]);
    }
}
