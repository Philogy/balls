use balls::parser::{
    ast::{Ast, Macro},
    lexer, parser,
};
use chumsky::Parser;

fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();

    let lex_out = lexer::lex(src.as_str());

    let tokens = lex_out.unwrap();

    for tok in tokens.iter() {
        println!("tok: {:?}", tok);
    }

    let parse_out = parser::parse_tokens(tokens.into_iter().map(|t| t.inner).collect());
    let ast_nodes = parse_out.unwrap();

    for node in ast_nodes {
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
                    println!("  {:?}", statement);
                }
            }
            node => {
                println!("node: {:?}", node);
            }
        }
    }
}
