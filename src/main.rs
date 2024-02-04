use balls::parser::lexer;
use chumsky::Parser;

fn main() {
    let src = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();

    let out = lexer::lex(src.as_str());

    println!("{:?}", out);
}
