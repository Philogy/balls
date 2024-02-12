use balls::parser::{error_printing::print_errors, lexer, parser, types::resolve_span_span};
use balls::schedulers::BackwardsMachine;
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

            // println!("inputs:");

            // for (id, ident) in macro_def.inputs.iter().enumerate() {
            //     println!("{}: {}", id, ident);
            // }

            // for (node, res) in tmacro.nodes {
            //     println!("\n");
            //     println!("res: {:?}", res);
            //     dbg!(node);
            // }

            // println!("output_nodes: {:?}", tmacro.output_ids);

            let machine: BackwardsMachine = tmacro.into();

            dbg!(machine);
        }
    }
}
