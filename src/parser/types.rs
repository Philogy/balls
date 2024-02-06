use std::fmt::Debug;

pub type Ident = String;

pub type Span = std::ops::Range<usize>;

#[derive(Debug, Clone)]
pub struct Spanned<T: Debug + Clone> {
    pub inner: T,
    pub span: Span,
}

impl<T: Debug + Clone> Spanned<T> {
    pub fn map<F, U: Debug + Clone>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned::new(f(self.inner), self.span)
    }
}

impl<T: Debug + Clone> Spanned<T> {
    pub fn new(inner: T, span: Span) -> Self {
        Spanned { inner, span }
    }

    pub fn get_text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.span.clone()]
    }
}

pub fn resolve_span_span<T: Clone + Debug>(span_span: &Span, spans: &Vec<Spanned<T>>) -> Span {
    spans[span_span.start].span.start..spans[span_span.end - 1].span.end
}
