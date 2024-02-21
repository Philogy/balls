use std::fmt::Debug;
use std::iter::zip;

#[derive(Debug)]
pub struct Swapper<'a, T: Eq> {
    from: &'a mut Vec<T>,
    to: &'a [T],
    incorrect_indices: Vec<usize>,
}

/// Iterator struct running over the O(n^2) algorithm to determine the optimal amount of
/// swaps to transform a given input stack ordering into an output ordering.
impl<'a, T: Eq> Swapper<'a, T> {
    pub fn new(from: &'a mut Vec<T>, to: &'a [T]) -> Self {
        // TODO: Remove panics and turn into result
        if from.len() != to.len() {
            panic!("length mismatch");
        }
        if from.len() == 0 {
            panic!("Length 0");
        }
        let incorrect_indices = (0..to.len() - 1).filter(|i| from[*i] != to[*i]).collect();
        Self {
            from,
            to,
            incorrect_indices,
        }
    }

    fn swap(&mut self, i: usize) -> usize {
        let last_idx = self.to.len() - 1;
        self.from.swap(i, last_idx);
        last_idx - i
    }

    pub fn done(&self) -> bool {
        let last_idx = self.to.len() - 1;
        self.incorrect_indices.len() == 0 && self.from[last_idx] == self.to[last_idx]
    }

    pub fn peek_next_swap(&self) -> Option<(usize, bool)> {
        let last_idx = self.from.len() - 1;

        let last = &self.from[last_idx];
        if last != &self.to[last_idx] {
            let (swap_to_idx, _) = zip(self.from.iter(), self.to)
                .enumerate()
                .take(last_idx)
                .filter(|(_, (x, y))| x != y && *y == last)
                .next()?;
            return Some((swap_to_idx, true));
        }

        let next_incorrect = self.incorrect_indices.last()?;
        Some((*next_incorrect, false))
    }

    /// Will return None if the iterator hasn't terminated yet and therefore it's unknowable.
    pub fn matching_count(&self) -> Option<bool> {
        match self.peek_next_swap() {
            None => Some(self.done()),
            Some(_) => None,
        }
    }

    pub fn get_swaps(&mut self) -> Vec<usize> {
        let mut swaps = vec![];
        while let Some(depth) = self.next() {
            swaps.push(depth);
        }
        swaps
    }
}

/// Iterators that returns swaps (by depth) to transform one into another.
impl<T: Eq> Iterator for Swapper<'_, T> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (swap_to_idx, is_correcting_swap) = self.peek_next_swap()?;
        if is_correcting_swap {
            let idx_idx = self
                .incorrect_indices
                .iter()
                .position(|x| *x == swap_to_idx)?;
            self.incorrect_indices.swap_remove(idx_idx);
        }
        Some(self.swap(swap_to_idx))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let total_incorrect = self.incorrect_indices.len();
        (total_incorrect, Some(total_incorrect * 2))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic_swaps() {
        let mut from = vec![4, 1, 2, 3];
        let to = vec![1, 2, 3, 4];

        let mut s = Swapper::new(&mut from, &to);

        assert!(!s.done());

        assert_eq!(s.matching_count(), None);
        assert_eq!(s.next(), Some(1));
        assert_eq!(s.next(), Some(2));
        assert_eq!(s.next(), Some(3));
        assert_eq!(s.next(), None);
        assert!(s.done());
        assert_eq!(s.matching_count(), Some(true));
    }

    #[test]
    fn test_two_cycle_swap() {
        let mut from = vec![5, 6, 4, 1, 2, 3];
        let to = vec![6, 5, 1, 2, 3, 4];

        let mut s = Swapper::new(&mut from, &to);

        assert!(!s.done());
        assert_eq!(s.matching_count(), None);
        assert_eq!(s.next(), Some(1));
        assert_eq!(s.next(), Some(2));
        assert_eq!(s.next(), Some(3));
        assert_eq!(s.next(), Some(4));
        assert_eq!(s.next(), Some(5));
        assert_eq!(s.next(), Some(4));
        assert_eq!(s.next(), None);
        assert!(s.done());
        assert_eq!(s.matching_count(), Some(true));
    }

    #[test]
    fn test_complete_swap() {
        let mut from = vec![1, 3, 4];
        let to = vec![1, 3, 4];

        let mut s = Swapper::new(&mut from, &to);

        assert!(s.done());
        assert_eq!(s.next(), None);
        assert_eq!(s.matching_count(), Some(true));
    }

    #[test]
    fn test_swaps_duplicate() {
        let mut from = vec![4, 4, 3, 2, 1];
        let to = vec![3, 4, 2, 1, 4];
        let mut s = Swapper::new(&mut from, &to);

        assert!(!s.done());
        assert_eq!(s.matching_count(), None);
        assert_eq!(s.next(), Some(1));
        assert_eq!(s.next(), Some(2));
        assert_eq!(s.next(), Some(4));
        assert_eq!(s.next(), None);
        assert!(s.done());
        assert_eq!(s.matching_count(), Some(true));
    }

    #[test]
    fn test_non_matching_count() {
        let mut from = vec![1, 1, 3];
        let to = vec![3, 1, 2];
        let mut s = Swapper::new(&mut from, &to);

        assert!(!s.done());
        assert_eq!(s.matching_count(), None);

        assert_eq!(s.next(), Some(2));
        assert_eq!(s.next(), None);

        assert_eq!(s.matching_count(), Some(false));
    }
}
