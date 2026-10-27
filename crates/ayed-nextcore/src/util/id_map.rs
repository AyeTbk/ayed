use std::{collections::HashMap, hash::Hash};

pub struct IdMap<K: IdMapKey, V> {
    inner: HashMap<K, V>,
    next_key: K,
}

impl<K: IdMapKey, V> IdMap<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, value: V) -> K {
        let key = self.next_key;
        self.next_key = key.next();
        self.inner.insert(key, value);
        key
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        self.inner.remove(&key)
    }
}

impl<K: IdMapKey, V> std::default::Default for IdMap<K, V> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            next_key: Default::default(),
        }
    }
}

pub trait IdMapKey: Copy + Default + Hash + Eq {
    fn next(&self) -> Self;
}

#[macro_export]
macro_rules! impl_IdMapKey {
    (for $ty:ident) => {
        impl crate::util::id_map::IdMapKey for $ty {
            fn next(&self) -> Self {
                let mut next = *self;
                next.id = next.id.checked_add(1).unwrap();
                next
            }
        }
    };
}

impl IdMapKey for u16 {
    fn next(&self) -> Self {
        *self + 1
    }
}
