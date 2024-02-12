pub trait Searchable<I: PartialEq> {
    fn index_of(&mut self, item: I) -> Option<usize>;

    fn contains(&mut self, item: I) -> bool {
        self.index_of(item).is_some()
    }

    fn total(&mut self, item: I) -> usize;
}

impl<I: PartialEq, T: Iterator<Item = I>> Searchable<I> for T {
    fn index_of(&mut self, item: I) -> Option<usize> {
        self.position(|el| el == item)
    }

    fn total(&mut self, item: I) -> usize {
        self.filter(|el| el == &item).count()
    }
}
