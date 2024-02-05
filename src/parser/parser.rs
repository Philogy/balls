use chumsky::prelude::*;

use crate::parser::{
    ast::{Ast, Expr, Macro, OpDefinition, Statement},
    tokens::Token,
    types::{Ident, Spanned},
};

fn get_ident() -> impl Parser<Token, Ident, Error = Simple<Token>> {
    select! { Token::Ident(ident) => ident }
}

fn dependency_definition() -> impl Parser<Token, Ast, Error = Simple<Token>> {
    just(Token::Dependency).ignore_then(get_ident().map(Ast::Dependency))
}

fn le_bytes_to_u16(bytes: Vec<u8>) -> u16 {
    match bytes.len() {
        0 => 0,
        1 => bytes[0] as u16,
        2 => u16::from_le_bytes([bytes[0], bytes[1]]),
        3.. => panic!("{:?} longer than 2 bytes", bytes),
    }
}

fn number_to_u16() -> impl Parser<Token, u16, Error = Simple<Token>> {
    select! { Token::Number(bytes) if bytes.len() <= 2 => le_bytes_to_u16(bytes) }
}

fn dependency_list(token: Token) -> impl Parser<Token, Vec<Ident>, Error = Simple<Token>> {
    just(token)
        .ignore_then(
            get_ident()
                .separated_by(just(Token::Comma))
                .delimited_by(just(Token::OpenRound), just(Token::CloseRound)),
        )
        .or_not()
        .map(Option::unwrap_or_default)
}

fn op_definition() -> impl Parser<Token, Ast, Error = Simple<Token>> {
    let op_def_head = just(Token::Op)
        .ignore_then(get_ident())
        .then_ignore(just(Token::Assign));

    let stack_io = just(Token::Stack).ignore_then(
        number_to_u16()
            .then_ignore(just(Token::Comma))
            .then(number_to_u16())
            .delimited_by(just(Token::OpenRound), just(Token::CloseRound)),
    );

    let reads_writes = dependency_list(Token::Reads).then(dependency_list(Token::Writes));

    op_def_head.then(stack_io).then(reads_writes).map(
        |((name, (stack_in, stack_out)), (reads, writes))| {
            Ast::OpDef(OpDefinition {
                name,
                stack_in,
                stack_out,
                reads,
                writes,
            })
        },
    )
}

fn expression() -> impl Parser<Token, Expr, Error = Simple<Token>> {
    recursive(|expr| {
        let arg_list = expr
            .separated_by(just(Token::Comma))
            .delimited_by(just(Token::OpenRound), just(Token::CloseRound))
            .map(Box::new);
        let call = get_ident()
            .then(arg_list)
            .map(|(name, args)| Expr::Call { name, args });
        let num = select! { Token::Number(bytes) => Expr::Num(bytes) };

        call.or(num).or(get_ident().map(Expr::Var))
    })
}

fn statement() -> impl Parser<Token, Statement, Error = Simple<Token>> {
    // my_var =
    let var_assign = get_ident().then_ignore(just(Token::Assign)).or_not();

    // sstore(caller(), add(sload(caller()), sub(0x34, x)))
    var_assign
        .then(expression())
        .map(|(ident, expr)| Statement { ident, expr })
}

fn macro_definition() -> impl Parser<Token, Ast, Error = Simple<Token>> {
    // macro TRANSFER =
    let macro_def_head = just(Token::Macro)
        .ignore_then(get_ident())
        .then_ignore(just(Token::Assign));

    // [a, b, c]
    let stack_parameters = || {
        get_ident()
            .separated_by(just(Token::Comma))
            .delimited_by(just(Token::OpenSquare), just(Token::CloseSquare))
    };

    // [a, b, c] ->
    let stack_in = stack_parameters()
        .then_ignore(just(Token::Arrow))
        .or_not()
        .map(Option::unwrap_or_default);

    // { var1 = op(a, ...) ... sstore(x, y)  }
    let body = statement()
        .repeated()
        .delimited_by(just(Token::OpenCurly), just(Token::CloseCurly));

    // -> [result, nice]
    let stack_out = just(Token::Arrow)
        .ignore_then(stack_parameters())
        .or_not()
        .map(Option::unwrap_or_default);

    macro_def_head
        .then(stack_in)
        .then(body)
        .then(stack_out)
        .map(|(((name, inputs), body), outputs)| {
            Ast::Macro(Macro {
                name,
                inputs,
                outputs,
                body,
            })
        })
}

pub fn parser() -> impl Parser<Token, Vec<Spanned<Ast>>, Error = Simple<Token>> {
    just(Token::Define)
        .ignore_then(
            dependency_definition()
                .or(op_definition())
                .or(macro_definition())
                .map_with_span(Spanned::new),
        )
        .repeated()
        .then_ignore(end())
}

pub fn parse_tokens(tokens: Vec<Token>) -> Result<Vec<Spanned<Ast>>, Vec<Simple<Token>>> {
    parser().parse(tokens)
}
