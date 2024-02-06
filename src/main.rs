use balls::parser::{
    ast::{Ast, Macro},
    error_printing::print_errors,
    lexer, parser,
    types::resolve_span_span,
};

fn main() {
    let file_path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&file_path).unwrap();

    let lex_out = lexer::lex(src.as_str());

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
        for node in ast_nodes {
            println!();
            match node.inner {
                Ast::Macro(Macro {
                    name,
                    inputs,
                    outputs,
                    body,
                }) => {
                    println!("name: {:?}", name);
                    println!("inputs: {:?}", inputs);
                    println!("outputs: {:?}", outputs);
                    println!("statements:");
                    for statement in body {
                        dbg!(statement);
                    }
                }
                node => {
                    println!("node: {:?}", node);
                }
            }
            println!("tokens: {:?}", &tokens[node.span]);
        }
    }
}
