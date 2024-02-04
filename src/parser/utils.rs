use std::fmt::Debug;

pub type Span = std::ops::Range<usize>;

#[derive(Debug)]
pub struct Spanned<T: Debug> {
    pub inner: T,
    pub span: Span,
}

impl<T: Debug> Spanned<T> {
    pub fn new(inner: T, span: Span) -> Self {
        Spanned { inner, span }
    }
}
