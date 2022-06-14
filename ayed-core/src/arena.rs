use std::{fmt::Debug, hash::Hash, marker::PhantomData};

pub struct Handle<T> {
    index: usize,
    _ghost: PhantomData<T>,
}

impl<T> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").field("id", &self.index).finish()
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            _ghost: self._ghost,
        }
    }
}
impl<T> Copy for Handle<T> {}
impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index.eq(&other.index)
    }
}
impl<T> Eq for Handle<T> {}
impl<T> Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state)
    }
}

#[derive(Debug)]
pub struct Arena<T> {
    elements: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            elements: Vec::default(),
        }
    }

    pub fn allocate(&mut self, t: T) -> Handle<T> {
        let index = self.elements.len();
        self.elements.push(t);
        Handle {
            index,
            _ghost: PhantomData,
        }
    }

    pub fn get(&self, handle: Handle<T>) -> &T {
        self.elements.get(handle.index).expect("bad handle")
    }

    pub fn get_mut(&mut self, handle: Handle<T>) -> &mut T {
        self.elements.get_mut(handle.index).expect("bad handle")
    }

    pub fn replace(&mut self, handle: Handle<T>, t: T) -> T {
        let dst = self.get_mut(handle);
        std::mem::replace(dst, t)
    }

    pub fn elements(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        ElementsIter {
            index: 0,
            elements: &self.elements,
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ElementsIter<'a, T> {
    index: usize,
    elements: &'a Vec<T>,
}

impl<'a, T> Iterator for ElementsIter<'a, T> {
    type Item = (Handle<T>, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.elements.get(index).map(|value| {
            (
                Handle {
                    index,
                    _ghost: PhantomData,
                },
                value,
            )
        })
    }
}
