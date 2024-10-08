use chumsky::prelude::*;
use num_bigint::BigUint;

use crate::parser::{tokens::Token, types::Spanned};

fn text(
    literal: &'static str,
    token: Token,
    label: &'static str,
) -> impl Parser<char, Token, Error = Simple<char>> {
    just(literal).to(token).labelled(label)
}

fn symbols() -> impl Parser<char, Token, Error = Simple<char>> {
    text("->", Token::Arrow, "arrow")
        .or(text("(", Token::OpenRound, "open round bracket"))
        .or(text(")", Token::CloseRound, "close round bracket"))
        .or(text("{", Token::OpenCurly, "open curly bracket"))
        .or(text("}", Token::CloseCurly, "close curly bracket"))
        .or(text("[", Token::OpenSquare, "open square bracket"))
        .or(text("]", Token::CloseSquare, "close square bracket"))
        .or(text(
            "<",
            Token::OpenAngle,
            "open angled bracket / less-than",
        ))
        .or(text(
            ">",
            Token::CloseAngle,
            "closed angled bracket / greater-than",
        ))
        .or(text(",", Token::Comma, "comma"))
        .or(text("=", Token::Assign, "assign"))
}

fn string_to_num<const BASE: u32>(s: String) -> Token {
    Token::Number(
        BigUint::parse_bytes(s.as_bytes(), BASE).expect("Lexer should've ensured only valid bytes"),
    )
}

fn number() -> impl Parser<char, Token, Error = Simple<char>> {
    let decimal = text::digits(10).map(string_to_num::<10>);
    let hexadecimal = just("0x")
        .ignore_then(text::digits(16))
        .map(string_to_num::<16>);

    hexadecimal.or(decimal)
}

fn ident() -> impl Parser<char, Token, Error = Simple<char>> {
    filter(|c: &char| c.is_ascii_alphabetic() || c == &'_')
        .map(Some)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| c.is_ascii_alphanumeric() || c == &'_').repeated(),
        )
        .chain::<char, Vec<_>, _>(filter(|c: &char| c == &'\'').repeated())
        .collect()
        .map(|name: String| match name.as_str() {
            "dependency" => Token::Dependency,
            "fn" => Token::Fn,
            "stack" => Token::Stack,
            "reads" => Token::Reads,
            "writes" => Token::Writes,
            "extern" => Token::External,
            "const" => Token::Const,
            _ => Token::Ident(name),
        })
}

pub fn lexer() -> impl Parser<char, Vec<Spanned<Token>>, Error = Simple<char>> {
    let single_line_comment = just("//").then(take_until(text::newline())).to(());

    let multi_line_comment = just("/*").then(take_until(just("*/"))).to(());

    let comment = single_line_comment
        .or(multi_line_comment)
        .padded()
        .labelled("comment");

    let token = symbols().or(number()).or(ident());

    token
        .map_with_span(Spanned::new)
        .padded_by(comment.repeated())
        .padded()
        .repeated()
        .then_ignore(end())
}

pub fn lex(source: &str) -> Result<Vec<Spanned<Token>>, Vec<Simple<char>>> {
    lexer().parse(source)
}
