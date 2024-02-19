use crate::comp_graph::Computation;
use crate::scheduling::Step;
use crate::transformer::TransformedMacro;

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

pub fn format_with_stack_comments(
    tmacro: &TransformedMacro,
    steps: Vec<Step>,
    comment_start: usize,
    indent: usize,
) -> String {
    let mut out = String::new();

    let main_width = comment_start - indent;
    let indent = " ".repeat(indent);

    let mut stack: Vec<String> = tmacro
        .nodes
        .iter()
        .filter_map(|(_, comp)| match comp {
            Computation::TopLevelInput(ident) => Some(ident.clone()),
            _ => None,
        })
        .collect();

    let line = format!(
        "{indent}{:<width$}[{}]",
        "// takes:",
        stack.join(","),
        width = main_width + 3
    );
    out.push_str(&line);
    out.push('\n');

    for step in steps {
        let step_repr = match step {
            Step::Op(id) => match &tmacro.nodes[id].1 {
                Computation::Op(ident) => ident.clone(),
                Computation::External(ident) => format!("{}()", ident),
                Computation::TopLevelInput(ident) => ident.clone(),
                Computation::Const(num) => format!("0x{:x}", num),
            },
            Step::Dup(depth) => format!("dup{}", depth),
            Step::Swap(depth) => format!("swap{}", depth),
            Step::Pop => "pop".to_string(),
        };
        match step {
            Step::Op(id) => {
                let mut args = vec![];
                let node = &tmacro.nodes[id].0;
                for _ in 0..node.operands.len() {
                    args.push(stack.pop().expect("Invalid instruction sequence"));
                }
                let assignment = tmacro
                    .assignments
                    .iter()
                    .find(|(_, statement_id)| *statement_id == id);
                match (assignment, node.has_output) {
                    (Some((ident, _)), true) => {
                        stack.push(ident.clone());
                    }
                    (None, true) => match args.len() {
                        0 => stack.push(step_repr.clone()),
                        _ => stack.push(format!("{}({})", step_repr, args.join(", "))),
                    },
                    (None, false) => {}
                    _ => panic!("Found assignment but comp node.has_output reported as f"),
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
        let lone_line = format!("{indent}{}", step_repr);
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
        stack.join(","),
        width = main_width + 3
    );
    out.push_str(&line);
    out.push('\n');

    out
}
