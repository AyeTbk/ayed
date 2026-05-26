use std::{borrow::Borrow, collections::HashMap, hash::Hash};

#[derive(Debug, Default, Clone)]
pub struct InsertOrderMap<K, V> {
    order: HashMap<K, usize>,
    entries: Vec<(K, V)>,
}

impl<K: Eq + Hash + Clone, V> InsertOrderMap<K, V> {
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(k).is_some()
    }

    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = *self.order.get(&k)?;
        self.entries.get(idx).map(|(_, v)| v)
    }

    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let idx = *self.order.get(&k)?;
        self.entries.get_mut(idx).map(|(_, v)| v)
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let maybe_previous = self.remove(&k);
        let idx = self.entries.len();
        self.entries.push((k.clone(), v));
        self.order.insert(k, idx);
        maybe_previous
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let existing_idx = self.order.remove(&k)?;
        let (_, previous) = self.entries.remove(existing_idx);
        self.order = self
            .entries
            .iter()
            .enumerate()
            .map(|(idx, (k, _))| (k.clone(), idx))
            .collect();
        Some(previous)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

impl<K, V> Extend<(K, V)> for InsertOrderMap<K, V>
where
    K: Eq + Hash + Clone,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        // PERF: this could be made better by removing from entries first, and after
        // being done with iter, then rebuild the hashmap.
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}
