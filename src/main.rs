use balls::comp_graph::Computation;
use balls::parser::{error_printing::print_errors, lexer, parser, types::resolve_span_span};
use balls::scheduling::astar::AStarScheduler;
use balls::scheduling::schedulers::Guessooor;
use balls::scheduling::{BackwardsMachine, Step};
use balls::transformer::GlobalContext;
use std::time::Instant;

fn main() {
    let file_path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&file_path).unwrap();

    let start = Instant::now();
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
    let parse_lex_time = start.elapsed().as_micros() as f64 / 1000.0;

    if let Some(ast_nodes) = maybe_ast_nodes {
        let start = Instant::now();
        let ctx = GlobalContext::from(ast_nodes);

        let macro_def = ctx.macros.first().expect("No macro to schedule");
        println!("Macro {:?}", macro_def.name);

        let tmacro = ctx.transform(macro_def.clone());

        let machine: BackwardsMachine = tmacro.clone().into();
        let preprocessing_time = start.elapsed().as_micros() as f64 / 1000.0;

        let start = Instant::now();
        let (steps, (cost, total, capacity_est)) = Guessooor::new(0.035).schedule(machine);
        let schedule_time = start.elapsed().as_secs_f64();

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
            println!("  {}", s);
        }
        println!();
        if ctx.macros.len() > 1 {
            println!("WARNING: More than 1 macro found, only scheduling one at a time for now");
        }
        println!("Lexing + parsing: {:.2} ms", parse_lex_time);
        println!("Macro pre-processing: {:.2} ms", preprocessing_time);
        if schedule_time < 0.25 {
            println!("\nScheduling: {:.1} ms", schedule_time * 1000.0);
        } else {
            println!("\nScheduling: {:.3} s", schedule_time);
        }
        println!(
            "explored: {} ({:.0} / s)",
            total,
            total as f64 / schedule_time
        );
        println!("cost: {}", cost);
        if capacity_est < 0.0 {
            let factor_off = 1.0 / (capacity_est + 1.0);
            if factor_off >= 3.0 {
                println!("Underestimated explored nodes by: {:.2}x", factor_off);
            } else {
                println!(
                    "Underestimated explored nodes by: {:.2}%",
                    capacity_est * -100.0
                );
            }
        } else {
            let factor_off = capacity_est + 1.0;
            if factor_off >= 3.0 {
                println!("Overestimated explored nodes by: {:.2}x", factor_off);
            } else {
                println!(
                    "Overestimated explored nodes by: {:.2}%",
                    capacity_est * 100.0
                );
            }
        }
    }
}
