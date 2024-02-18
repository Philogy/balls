use balls::huff_formatter;
use balls::parser::{error_printing::print_errors, lexer, parser, types::resolve_span_span};
use balls::scheduling::astar::AStarScheduler;
use balls::scheduling::schedulers::Guessooor;
use balls::scheduling::{BackwardsMachine, ScheduleInfo};
use balls::transformer::GlobalContext;
use balls::TimeDelta;
use clap::Parser;
use std::time::Instant;

const DEFAULT_GUESSOR_FACTOR: f32 = 0.035;
const DEFAULT_COMMENT_START: usize = 32;

#[derive(Parser)]
#[clap(
    author = "philogy",
    version = "0.0.1",
    about = "BALLS is a light-weight DSL aimed at giving experts full control over their bytecode while providing some zero-cost abstractions like variable assignments."
)]
struct Cli {
    #[clap(default_value = "ma.balls")]
    file_path: String,

    #[clap(short, long)]
    dijkstra: bool,

    #[clap(short, long, default_value_t=DEFAULT_GUESSOR_FACTOR)]
    guess: f32,

    #[clap(short, long, default_value_t=DEFAULT_COMMENT_START, help="Character offset at which the // for the stack comment starts")]
    comment: usize,

    #[clap(short, long, default_value_t = 4)]
    indent: usize,
}

fn main() {
    let args = Cli::parse();
    let file_path = &args.file_path;
    let src = std::fs::read_to_string(file_path).unwrap();

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
    let parse_lex_time = start.elapsed().as_secs_f64();

    if let Some(ast_nodes) = maybe_ast_nodes {
        let start = Instant::now();
        let ctx = GlobalContext::from(ast_nodes);

        let macro_def = ctx.macros.first().expect("No macro to schedule");
        println!("Macro {:?}", macro_def.name);

        let tmacro = ctx.transform(macro_def.clone());

        let machine: BackwardsMachine = tmacro.clone().into();
        let nodes: Vec<_> = tmacro.nodes.iter().map(|(node, _)| node.clone()).collect();

        let preprocessing_time = start.elapsed().as_secs_f64();

        // Schedule and measure time elapsed.
        let start = Instant::now();
        let (steps, (cost, total, capacity_est)) = Guessooor::new(args.guess).schedule(
            ScheduleInfo {
                nodes: nodes.as_slice(),
                target_input_stack: tmacro.input_ids.as_slice(),
            },
            machine,
        );
        let schedule_time = start.elapsed().as_secs_f64();

        let output =
            huff_formatter::format_with_stack_comments(&tmacro, steps, args.comment, args.indent);
        println!("{}", output);

        println!();
        println!("Lexing + parsing: {}", parse_lex_time.humanize_seconds());
        println!(
            "Macro pre-processing: {}",
            preprocessing_time.humanize_seconds()
        );
        println!("\nScheduling: {}", schedule_time.humanize_seconds());
        println!(
            "explored: {} ({:.0} / s)",
            total,
            total as f64 / schedule_time
        );
        println!("cost: {}", cost);
        let (is_pos, fmt_factor) = capacity_est.humanize_factor();
        if is_pos {
            println!("Overestimated explored nodes by: {}", fmt_factor);
        } else {
            println!("Underestimated explored nodes by: {}", fmt_factor);
        }
        if ctx.macros.len() > 1 {
            println!(
                "TODO-WARNING: More than 1 macro found, only scheduling one at a time for now"
            );
        }
    }
}
