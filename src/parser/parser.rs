use super::utils::{OrDefaultParser, TokenParser};
use chumsky::prelude::*;
use num_bigint::{BigUint, TryFromBigIntError};

use crate::parser::{
    ast::{Ast, Expr, Function, HuffMacro, MacroArg, Statement},
    tokens::Token,
    types::Spanned,
};

fn ident() -> impl Parser<Token, String, Error = Simple<Token>> {
    select! { Token::Ident(ident) => ident }
}

fn dependency_definition() -> impl Parser<Token, Ast, Error = Simple<Token>> {
    just(Token::Dependency).ignore_then(ident().map(Ast::Dependency))
}

fn stack_size() -> impl Parser<Token, u16, Error = Simple<Token>> {
    select! { Token::Number(num) => num }.validate(|num, span, emit| match num.try_into() {
        Ok(lol) => lol,
        Err(err) => {
            let err: TryFromBigIntError<BigUint> = err;
            emit(Simple::custom(
                span,
                format!(
                    "Number {} exceeds max valid stack size specifier (max: {})",
                    err.into_original(),
                    u16::MAX
                ),
            ));
            u16::MAX
        }
    })
}

fn dependency_list(
    token: Token,
) -> impl Parser<Token, Vec<Spanned<String>>, Error = Simple<Token>> {
    just(token)
        .ignore_then(
            ident()
                .map_with_span(Spanned::new)
                .list()
                .delimited_by(just(Token::OpenRound), just(Token::CloseRound)),
        )
        .or_default()
}

fn recover_for_delimiters<T, const N: usize>(
    open: Token,
    close: Token,
    other_delims: [(Token, Token); N],
    parser: impl Parser<Token, T, Error = Simple<Token>>,
) -> impl Parser<Token, Result<T, ()>, Error = Simple<Token>> {
    parser
        .delimited_by(just(open.clone()), just(close.clone()))
        .map(Ok)
        .recover_with(nested_delimiters(open, close, other_delims, |_| Err(())))
}

fn recover_for_round_delimited<T>(
    parser: impl Parser<Token, T, Error = Simple<Token>>,
) -> impl Parser<Token, Result<T, ()>, Error = Simple<Token>> {
    recover_for_delimiters(
        Token::OpenRound,
        Token::CloseRound,
        [
            (Token::OpenSquare, Token::CloseSquare),
            (Token::OpenCurly, Token::CloseCurly),
            (Token::OpenAngle, Token::CloseAngle),
        ],
        parser,
    )
}

fn stack_io() -> impl Parser<Token, Result<(u16, u16), ()>, Error = Simple<Token>> {
    just(Token::Stack).ignore_then(recover_for_round_delimited(
        stack_size()
            .then_ignore(just(Token::Comma))
            .then(stack_size()),
    ))
}

fn interactions() -> impl Parser<
    Token,
    Result<(u16, u16, Vec<Spanned<String>>, Vec<Spanned<String>>), ()>,
    Error = Simple<Token>,
> {
    stack_io()
        .then(dependency_list(Token::Reads))
        .then(dependency_list(Token::Writes))
        .map(|((stack_io_res, reads), writes)| {
            stack_io_res.map(|(stack_in, stack_out)| (stack_in, stack_out, reads, writes))
        })
}

fn number() -> impl Parser<Token, BigUint, Error = Simple<Token>> {
    select! { Token::Number(num) => num }.validate(|num, span, emit| {
        if num.bits() <= 256 {
            num
        } else {
            emit(Simple::custom(
                span,
                format!("Expression constant 0x{:x} larger than 32-bytes", num),
            ));
            BigUint::from_bytes_le(&[0xff; 32])
        }
    })
}

fn macro_arg() -> impl Parser<Token, MacroArg, Error = Simple<Token>> {
    ident()
        .map(MacroArg::ArgRef)
        .or(number().map(MacroArg::Num))
}

fn expression() -> impl Parser<Token, Spanned<Expr>, Error = Simple<Token>> {
    recursive(|expr| {
        let macro_args = macro_arg()
            .map_with_span(Spanned::new)
            .list()
            .delimited_by(just(Token::OpenAngle), just(Token::CloseAngle))
            .or_not()
            .map_with_span(Spanned::new);
        let stack_args = expr
            .list()
            .delimited_by(just(Token::OpenRound), just(Token::CloseRound))
            .map(Box::new)
            .map_with_span(Spanned::new);
        let call = ident()
            .map_with_span(Spanned::new)
            .then(macro_args)
            .then(stack_args)
            .map(|((ident, macro_args), stack_args)| Expr::Call {
                ident,
                macro_args: macro_args.map(|inner| inner.unwrap_or_default()),
                stack_args,
            });
        let num = number().map(Expr::Num);

        let var = ident().map(Expr::Var);

        call.or(num).or(var).map_with_span(Spanned::new)
    })
}

fn statement() -> impl Parser<Token, Statement, Error = Simple<Token>> {
    // Parses "my_var ="
    let var_assign = ident()
        .then_ignore(just(Token::Assign))
        .or_not()
        .map_with_span(|maybe_var, span| maybe_var.map(|ident| Spanned::new(ident, span)));

    // Parses "sstore(caller(), add(sload(caller()), x))" or
    // "wow = lmao(x, d)"
    var_assign
        .then(expression())
        .map(|(ident, expr)| Statement { ident, expr })
        .validate(|stated, span, emit| {
            if stated.ident.is_none() && !matches!(stated.expr.inner, Expr::Call { .. }) {
                emit(Simple::custom(
                    span,
                    format!("Top-level expression not allowed"),
                ))
            }
            stated
        })
}

fn stack_parameters() -> impl Parser<Token, Vec<Spanned<String>>, Error = Simple<Token>> {
    ident()
        .map_with_span(Spanned::new)
        .list()
        .delimited_by(just(Token::OpenSquare), just(Token::CloseSquare))
}

fn function_definition() -> impl Parser<Token, Ast, Error = Simple<Token>> {
    // fn TRANSFER
    let macro_def = just(Token::Fn).ignore_then(ident());

    // (arg1, arg2, ...)
    let macro_args = recover_for_round_delimited(ident().map_with_span(Spanned::new).list());

    // reads(...) writes(...)
    let reads_writes = dependency_list(Token::Reads).then(dependency_list(Token::Writes));

    // [a, b, c] -> [d, e]
    let stack_in_out = stack_parameters()
        .then_ignore(just(Token::Arrow))
        .then(stack_parameters().or_default());

    // { var1 = op(a, ...) ... sstore(x, y)  }
    let body = statement()
        .repeated()
        .delimited_by(just(Token::OpenCurly), just(Token::CloseCurly));

    macro_def
        .then(macro_args)
        .then(reads_writes)
        .then(stack_in_out)
        .then(body)
        .map(
            |((((ident, maybe_macro_args), (reads, writes)), (inputs, outputs)), body)| {
                maybe_macro_args
                    .map(|macro_args| {
                        Ast::Function(Function {
                            ident,
                            macro_args,
                            inputs,
                            outputs,
                            body,
                            reads,
                            writes,
                        })
                    })
                    .unwrap_or(Ast::Error)
            },
        )
}

fn extern_huff_macro_definition() -> impl Parser<Token, Ast, Error = Simple<Token>> {
    just(Token::External)
        .ignore_then(ident())
        .then(recover_for_round_delimited(
            ident().map_with_span(Spanned::new).list(),
        ))
        .then(interactions())
        .map(|((ident, maybe_macro_args), interactions)| {
            let macro_args = maybe_macro_args?;
            let (stack_in, stack_out, reads, writes) = interactions?;
            Ok(Ast::HuffMacro(HuffMacro {
                ident,
                macro_args,
                stack_in,
                stack_out,
                reads,
                writes,
            }))
        })
        .map(|maybe_ast: Result<Ast, ()>| maybe_ast.unwrap_or(Ast::Error))
}

fn extern_const_definition() -> impl Parser<Token, Ast, Error = Simple<Token>> {
    just(Token::Const).ignore_then(ident()).map(Ast::Const)
}

pub fn parser() -> impl Parser<Token, Vec<Spanned<Ast>>, Error = Simple<Token>> {
    dependency_definition()
        .or(extern_huff_macro_definition())
        .or(extern_const_definition())
        .or(function_definition())
        .map_with_span(Spanned::new)
        .repeated()
        .then_ignore(end())
}

pub fn parse_tokens(tokens: Vec<Token>) -> (Option<Vec<Spanned<Ast>>>, Vec<Simple<Token>>) {
    parser().parse_recovery(tokens)
}
