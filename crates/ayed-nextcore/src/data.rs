use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

#[derive(Default)]
pub struct Storage {
    USE REFCELLS AND EXPOSE GUARDS. DRILL LIFETIMES THROUGH
    inner: HashMap<TypeId, Box<dyn Any>>,
}

impl Storage {
    pub fn get<'a, A: Args<'a>>(&'a mut self) -> A::Output {
        todo!()
    }
}

pub trait Args<'a> {
    type Output;

    fn extract(storage: &'a mut Storage) -> Self::Output;
}

impl<'a, T: 'a + Any> Args<'a> for (T,) {
    type Output = (&'a mut T,);

    fn extract(storage: &'a mut Storage) -> Self::Output {
        let [t] = storage.inner.get_disjoint_mut([&TypeId::of::<T>()]);
        (t.unwrap().downcast_mut().unwrap(),)
    }
}

impl<'a, T: 'a + Any, U: 'a + Any> Args<'a> for (T, U) {
    type Output = (&'a mut T, &'a mut U);

    fn extract(storage: &'a mut Storage) -> Self::Output {
        let [t, u] = storage
            .inner
            .get_disjoint_mut([&TypeId::of::<T>(), &TypeId::of::<U>()]);
        (
            t.unwrap().downcast_mut().unwrap(),
            u.unwrap().downcast_mut().unwrap(),
        )
    }
}
