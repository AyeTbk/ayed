use std::{hash::Hash, marker::PhantomData};

pub struct Handle<V> {
    id: u32,
    generation: u32,
    _ghost: PhantomData<fn() -> V>,
}

impl<V> Handle<V> {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }
}
impl<V> std::fmt::Debug for Handle<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Handle(id: {}, gen: {})",
            self.id, self.generation
        ))
    }
}
impl<V> Clone for Handle<V> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            generation: self.generation,
            _ghost: PhantomData,
        }
    }
}
impl<V> Copy for Handle<V> {}
impl<V> PartialEq for Handle<V> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id) && self.generation.eq(&other.generation)
    }
}
impl<V> Eq for Handle<V> {}
impl<V> PartialOrd for Handle<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id
            .partial_cmp(&other.id)
            .or(self.generation.partial_cmp(&other.generation))
    }
}
impl<V> Ord for Handle<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other)
            .expect("Handle's fields should be Ord")
    }
}
impl<V> Hash for Handle<V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.generation.hash(state);
    }
}

struct Slot<V> {
    generation: u32,
    element: Option<V>,
}

pub struct SlotMap<V, K = V> {
    slots: Vec<Slot<V>>,
    free_slots: Vec<u32>,
    _key_type: PhantomData<fn() -> K>,
}

impl<V, K> SlotMap<V, K> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_slots: Vec::new(),
            _key_type: PhantomData,
        }
    }

    pub fn insert(&mut self, v: V) -> Handle<K> {
        let (id, generation) = if let Some(slot_id) = self.free_slots.pop() {
            let slot = &mut self.slots[slot_id as usize];
            debug_assert!(slot.element.is_none());
            slot.element = Some(v);
            (slot_id, slot.generation)
        } else {
            let slot_id = self.slots.len().try_into().unwrap();
            let generation = 0;
            self.slots.push(Slot {
                element: Some(v),
                generation,
            });
            (slot_id, generation)
        };

        Handle {
            id,
            generation,
            _ghost: PhantomData,
        }
    }

    pub fn remove(&mut self, k: Handle<K>) -> V {
        let slot = self
            .slots
            .get_mut(k.id as usize)
            .filter(|slot| slot.generation == k.generation)
            .ok_or(StaleHandleError)
            .unwrap();
        let maybe_value = slot.element.take().ok_or(StaleHandleError);
        slot.generation = slot.generation.checked_add(1).unwrap();
        self.free_slots.push(k.id);
        maybe_value.unwrap()
    }

    pub fn get(&self, k: Handle<K>) -> &V {
        self.slots
            .get(k.id as usize)
            .filter(|slot| slot.generation == k.generation)
            .and_then(|slot| slot.element.as_ref())
            .ok_or(StaleHandleError)
            .unwrap()
    }

    pub fn get_mut(&mut self, k: Handle<K>) -> &mut V {
        self.slots
            .get_mut(k.id as usize)
            .filter(|slot| slot.generation == k.generation)
            .and_then(|slot| slot.element.as_mut())
            .ok_or(StaleHandleError)
            .unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle<K>, &V)> + '_ {
        self.slots.iter().enumerate().filter_map(|(id, slot)| {
            slot.element.as_ref().map(|v| {
                (
                    Handle {
                        id: id as u32,
                        generation: slot.generation,
                        _ghost: PhantomData,
                    },
                    v,
                )
            })
        })
    }

    pub fn keys(&self) -> impl Iterator<Item = Handle<K>> + '_ {
        self.iter().map(|(k, _)| k)
    }
}

impl<V, K> Default for SlotMap<V, K> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct StaleHandleError;
