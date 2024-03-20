use crate::parser::tokens::Token;
use chumsky::{
    combinator::{Map, OrNot},
    primitive::just,
    Error, Parser,
};

pub trait TokenParser<O, E: Error<Token>>: Parser<Token, O, Error = E> + Sized {
    /// Equivalent to `.separated_by(just(Token::Comma))`
    fn list(self) -> impl Parser<Token, Vec<O>, Error = E> {
        self.separated_by(just(Token::Comma))
            .then_ignore(just(Token::Comma).or_not())
    }
}

impl<O, E: Error<Token>, P: Parser<Token, O, Error = E>> TokenParser<O, E> for P {}

pub trait OrDefaultParser<I: Clone, O: Default, E: Error<I>>:
    Parser<I, O, Error = E> + Sized
{
    #[allow(clippy::type_complexity)]
    fn or_default(self) -> Map<OrNot<Self>, fn(Option<O>) -> O, Option<O>> {
        self.or_not().map(Option::unwrap_or_default)
    }
}

impl<I: Clone, O: Default, E: Error<I>, P: Parser<I, O, Error = E>> OrDefaultParser<I, O, E> for P {}
