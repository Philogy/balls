use crate::parser::ast::{Function, MacroArg};
use crate::scheduling::ir::{CompNode, CompNodeId};
use crate::scheduling::Step;
use crate::transformer::ir_gen::ValueSource;

/// Minimum character width for the comment start such that at least the ending "// returns: [..."
/// is nicely formatted.
const MIN_EXTRA_SIZE: usize = 12;

pub fn validate_format_params(comment_start: usize, indent: usize) -> Option<String> {
    let min_comment_start = indent + MIN_EXTRA_SIZE;
    if comment_start < min_comment_start {
        Some(format!(
            "Specified comment start ({}) below minimum ({})",
            comment_start, min_comment_start
        ))
    } else {
        None
    }
}

// Huff macro arguments can be: opcodes, constants, macro_args

pub fn format_with_stack_comments(
    func: &Function,
    nodes: &[CompNode],
    sources: &[ValueSource],
    assignments: &[(String, CompNodeId)],
    steps: Vec<Step>,
    comment_start: usize,
    indent: usize,
) -> String {
    let mut out = format!(
        "#define macro {}({}) = takes({}) returns({}) {{\n",
        func.ident,
        func.macro_args
            .iter()
            .map(|spanned| spanned.inner.clone())
            .collect::<Vec<String>>()
            .join(","),
        func.inputs.len(),
        func.outputs.len()
    );

    let main_width = comment_start - indent;
    let indent = " ".repeat(indent);

    let mut stack: Vec<String> = func
        .inputs
        .iter()
        .map(|spanned| spanned.inner.clone())
        .collect();

    let line = format!(
        "{indent}{:<width$}[{}]",
        "// takes:",
        stack.join(", "),
        width = main_width + 3
    );
    out.push_str(&line);
    out.push('\n');

    for step in steps {
        let op_repr = match step {
            Step::Comp(id) => sources[id].huff_repr(),
            Step::Dup(depth) => format!("dup{}", depth),
            Step::Swap(depth) => format!("swap{}", depth),
            Step::Pop => "pop".into(),
        };
        match step {
            Step::Comp(id) => {
                let mut args = vec![];
                let node = &nodes[id];
                for _ in 0..node.operands.len() {
                    args.push(stack.pop().expect("Invalid instruction sequence"));
                }
                let assignment = assignments
                    .iter()
                    .find(|(_, statement_id)| *statement_id == id);
                if node.produces_value {
                    let value_repr = match assignment {
                        Some((ident, _)) => ident.clone(),
                        None => match &sources[id] {
                            ValueSource::MacroInvoke(ident, macro_args) => {
                                format!(
                                    "{}<{}>({})",
                                    ident,
                                    macro_args
                                        .iter()
                                        .map(MacroArg::balls_repr)
                                        .collect::<Vec<String>>()
                                        .join(", "),
                                    args.join(",")
                                )
                            }
                            ValueSource::Op(ident) => format!("{}({})", ident, args.join(",")),
                            ValueSource::MacroArg(arg) => arg.balls_repr(),
                            ValueSource::HuffConst(ident) => ident.clone(),
                            ValueSource::TopLevelInput(_) => panic!(
                                "Invalid instruction sequence, top-level-input cannot be comp"
                            ),
                        },
                    };
                    stack.push(value_repr);
                }
            }
            Step::Dup(depth) => {
                stack.push(stack[stack.len() - depth].clone());
            }
            Step::Swap(depth) => {
                let last_idx = stack.len() - 1;
                stack.swap(last_idx, last_idx - depth);
            }
            Step::Pop => {
                stack.pop();
            }
        }
        let lone_line = format!("{indent}{}", op_repr);
        let stack_repr = if stack.len() > 17 {
            format!("[..., {}]", stack[stack.len() - 17..].join(", "))
        } else {
            format!("[{}]", stack.join(", "))
        };
        // +1 accounts for the space between the op representation and the stack comment.
        let line = if lone_line.len() + 1 >= comment_start {
            format!(
                "{}\n{indent}//{}{}",
                lone_line,
                " ".repeat(main_width + 1),
                stack_repr
            )
        } else {
            format!("{:<comment_start$}// {}", lone_line, stack_repr)
        };
        out.push_str(&line);
        out.push('\n');
    }

    let line = format!(
        "{indent}{:<width$}[{}]",
        "// returns:",
        stack.join(", "),
        width = main_width + 3
    );
    out.push_str(&line);
    out.push('\n');

    out.push_str("}");

    out
}
