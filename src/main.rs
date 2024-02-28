use balls::huff_formatter;
use balls::parser::{error_printing::print_errors, lexer, parser, types::resolve_span_span};
use balls::scheduling::astar::AStarScheduler;
use balls::scheduling::schedulers::{Dijkstra, Guessooor};
use balls::scheduling::{BackwardsMachine, ScheduleInfo};
use balls::transformer::analysis::{validate_and_get_symbols, Symbol, Symbols};
use balls::transformer::ir_gen::gen_ir;
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

    #[clap(short, long, default_value_t = 1024)]
    max_stack_depth: usize,

    #[clap(short, long, help = "The path to which to write the output")]
    output_path: Option<String>,
}

const BALLS_INSERT_START: &str = "\n// balls-insert-start\n";
const BALLS_INSERT_END: &str = "\n// balls-insert-end\n";

fn splice_into_huff(path: &str, content_to_inject: String) -> Result<(), String> {
    let content =
        std::fs::read_to_string(&path).map_err(|_| format!("Failed to read file {}", path))?;
    let start_index = content
        .find(BALLS_INSERT_START)
        .ok_or(format!("Could not find \"{}\"", BALLS_INSERT_START))?
        + BALLS_INSERT_START.len();
    let end_index = content
        .find(BALLS_INSERT_END)
        .ok_or(format!("Could not find \"{}\"", BALLS_INSERT_END))?;

    let new_content = format!(
        "{}{}{}",
        &content[..start_index],
        content_to_inject,
        &content[end_index..]
    );

    std::fs::write(&path, new_content).map_err(|_| format!("Failed to write file {}", path))?;

    Ok(())
}

fn main() {
    let args = Cli::parse();

    assert!(
        args.max_stack_depth <= 1024,
        "TODO: Invalid max stack depth of {}",
        args.max_stack_depth
    );

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
        let symbols: Symbols = validate_and_get_symbols(ast_nodes).unwrap();

        let mut ball_macros: Vec<String> = Vec::new();

        let schedule_summaries: Vec<_> = symbols
            .values()
            .filter_map(|symbol| match &symbol.inner {
                Symbol::Function(func) => Some(func),
                _ => None,
            })
            .map(|func| {
                let (ir_graph, value_sources, assignments) = gen_ir(func, &symbols);

                let start = Instant::now();
                let preprocessing_time = start.elapsed().as_secs_f64();

                let (steps, tracker) = if args.dijkstra {
                    Dijkstra.schedule(&ir_graph, args.max_stack_depth)
                } else {
                    Guessooor::new(args.guess).schedule(&ir_graph, args.max_stack_depth)
                };

                let output = huff_formatter::format_with_stack_comments(
                    func,
                    ir_graph.nodes.as_slice(),
                    value_sources.as_slice(),
                    assignments.as_slice(),
                    steps,
                    args.comment,
                    args.indent,
                );

                ball_macros.push(output);

                (func.ident.clone(), tracker, preprocessing_time)
            })
            .collect();

        let full_balls = ball_macros.join("\n\n");
        match args.output_path {
            None => println!("{}", full_balls),
            Some(output_path) => {
                splice_into_huff(&output_path, full_balls).unwrap();
                println!("✅ Successfully inserted result into {}\n", &output_path);
            }
        }

        println!("Lexing + parsing: {}", parse_lex_time.humanize_seconds());
        for (name, tracker, preprocessing_time) in schedule_summaries {
            println!("{}:", name);
            println!(
                "  Macro pre-processing: {}",
                preprocessing_time.humanize_seconds()
            );
            tracker.report(2);
        }
    }
}
