pub trait AsIterator {
    type Item;
    type AsIter: Iterator<Item = Self::Item>;

    fn as_iter(self) -> Self::AsIter;
}

impl<'a> AsIterator for &'a str {
    type Item = char;
    type AsIter = std::str::Chars<'a>;

    fn as_iter(self) -> Self::AsIter {
        self.chars()
    }
}

impl<'a> AsIterator for &'a mut str {
    type Item = char;
    type AsIter = std::str::Chars<'a>;

    fn as_iter(self) -> Self::AsIter {
        self.chars()
    }
}

impl<'a> AsIterator for &'a String {
    type Item = char;
    type AsIter = std::str::Chars<'a>;

    fn as_iter(self) -> Self::AsIter {
        self.chars()
    }
}
