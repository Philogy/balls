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

pub trait TimeDelta {
    fn humanize_seconds(self) -> String;

    fn humanize_factor(self) -> (bool, String);
}

impl TimeDelta for f64 {
    fn humanize_seconds(self) -> String {
        match self {
            secs if secs >= 3600.0 => format!("{:.2}h", secs / 3600.0),
            secs if secs >= 60.0 => format!("{:.2}min", secs / 60.0),
            secs if secs < 0.2e-3 => format!("{:.2}μs", secs * 1e6),
            secs if secs < 0.2 => format!("{:.2}ms", secs * 1e3),
            secs => format!("{:.2}s", secs),
        }
    }

    fn humanize_factor(self) -> (bool, String) {
        let is_pos = self >= 1.0;
        let multiple = if is_pos { self } else { 1.0 / self };

        let formatted = if multiple >= 3.0 {
            format!("{:.1}x", multiple)
        } else {
            let fraction = if is_pos { multiple } else { 1.0 - self };

            format!("{:.2}%", fraction * 100.0)
        };

        (is_pos, formatted)
    }
}

pub trait CommaSeparatable {
    fn comma_sep(&self) -> String;
}

impl CommaSeparatable for usize {
    fn comma_sep(&self) -> String {
        self.to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .unwrap()
            .join(",")
    }
}
