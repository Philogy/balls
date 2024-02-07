use balls::ast_to_comp::{sort_ast_nodes, transform_macro, validate_and_extract_globals};
use balls::parser::{error_printing::print_errors, lexer, parser, types::resolve_span_span};

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
        let (dependencies, ops, macros) = sort_ast_nodes(ast_nodes);
        let (_, ops, macros) = validate_and_extract_globals(dependencies, ops, macros);

        for macro_def in macros {
            println!("Macro {:?}", macro_def.name);

            let (_, nodes, output_nodes, _) = transform_macro(&ops, macro_def.clone());

            println!("inputs:");

            for (id, ident) in macro_def.inputs.iter().enumerate() {
                println!("{}: {}", id, ident);
            }

            for (node, res) in nodes {
                println!("\n");
                println!("res: {:?}", res);
                dbg!(node);
            }

            println!("output_nodes: {:?}", output_nodes);
        }
    }
}
