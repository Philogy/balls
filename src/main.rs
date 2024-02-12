use balls::comp_graph::Computation;
use balls::parser::{error_printing::print_errors, lexer, parser, types::resolve_span_span};
use balls::scheduling::astar::AStarScheduler;
use balls::scheduling::dijkstra::Dijkstra;
use balls::scheduling::{BackwardsMachine, Step};
use balls::transformer::GlobalContext;

fn main() {
    let file_path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&file_path).unwrap();

    let lex_out = lexer::lex(src.as_str());

    // TODO: Proper lexer error handling
    let spanned_tokens = lex_out.unwrap();

    let tokens: Vec<_> = spanned_tokens.iter().map(|t| t.inner.clone()).collect();

    let (maybe_ast_nodes, errs) = parser::parse_tokens(tokens.clone());

    let errored = print_errors(&src, &file_path, errs, |tok_span| {
        resolve_span_span(tok_span, &spanned_tokens)
    });

    if errored {
        std::process::exit(1);
    }

    if let Some(ast_nodes) = maybe_ast_nodes {
        let ctx = GlobalContext::from(ast_nodes);

        for macro_def in ctx.macros.iter() {
            println!("Macro {:?}", macro_def.name);

            let tmacro = ctx.transform(macro_def.clone());

            let machine: BackwardsMachine = tmacro.clone().into();

            let (total, cost, steps) = Dijkstra::schedule(machine);

            for step in steps {
                let s = match step {
                    Step::Op(id) => match &tmacro.nodes[id].1 {
                        Computation::Op(ident) => ident.clone(),
                        Computation::TopLevelInput(ident) => ident.clone(),
                        Computation::Const(num) => format!("0x{:x}", num),
                    },
                    Step::Dup(depth) => format!("dup{}", depth),
                    Step::Swap(depth) => format!("swap{}", depth),
                    Step::Pop => "pop".to_string(),
                };
                println!("{}", s);
            }
            println!("total: {total}");
            println!("cost: {cost}");
        }
    }
}
