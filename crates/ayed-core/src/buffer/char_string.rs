use std::{io::Write, ops::Index, slice::SliceIndex};

#[derive(Debug, Clone, Default)]
pub struct CharString {
    inner: Vec<char>,
}

impl CharString {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub fn get(&self, idx: usize) -> Option<char> {
        self.inner.get(idx).copied()
    }

    pub fn push(&mut self, ch: char) {
        self.inner.push(ch);
    }

    pub fn pop(&mut self) -> Option<char> {
        self.inner.pop()
    }

    pub fn insert(&mut self, idx: usize, ch: char) {
        self.inner.insert(idx, ch);
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn remove(&mut self, idx: usize) -> char {
        self.inner.remove(idx)
    }

    pub fn extend(&mut self, other: CharString) {
        self.inner.extend(other.inner);
    }

    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.inner.iter().copied()
    }

    pub fn char_indices(&self) -> impl Iterator<Item = (usize, char)> + '_ {
        self.inner.iter().copied().enumerate()
    }

    pub fn find<P: CharPattern>(&self, pat: P) -> Option<usize> {
        pat.next_match(self).map(|(i, _)| i)
    }

    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, char>
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    pub fn write_all(&self, mut w: impl Write) -> std::io::Result<()> {
        let mut buf = [0u8; 4];
        for ch in &self.inner {
            let char_str = ch.encode_utf8(&mut buf);
            w.write_all(char_str.as_bytes())?;
        }
        Ok(())
    }
}

impl<S: AsRef<str>> From<S> for CharString {
    fn from(string: S) -> Self {
        Self {
            inner: string.as_ref().chars().collect(),
        }
    }
}

impl ToString for CharString {
    fn to_string(&self) -> String {
        let mut string = String::new();
        for ch in self.inner.iter().copied() {
            string.push(ch);
        }
        string
    }
}

impl<I> Index<I> for CharString
where
    I: SliceIndex<[char]>,
{
    type Output = <I as SliceIndex<[char]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl FromIterator<char> for CharString {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let mut this = Self::new();
        for ch in iter {
            this.push(ch);
        }
        this
    }
}

pub trait CharPattern {
    fn next_match(self, haystack: &CharString) -> Option<(usize, usize)>;
}

impl<T> CharPattern for T
where
    T: FnMut(char) -> bool,
{
    fn next_match(mut self, haystack: &CharString) -> Option<(usize, usize)> {
        for (i, ch) in haystack.char_indices() {
            if self(ch) {
                return Some((i, i + 1));
            }
        }

        None
    }
}
