use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
};

#[derive(Default)]
pub struct Storage {
    inner: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Storage {
    pub fn insert<T: Any>(&mut self, value: T) -> Option<T> {
        let wrapped: RefCell<Box<dyn Any>> = RefCell::new(Box::new(value));
        let old_wrapped = self.inner.insert(TypeId::of::<T>(), wrapped);
        let boxed = old_wrapped?
            .into_inner()
            .downcast::<T>()
            .expect("downcasting to T should be valid for key TypeId::of::<T>");
        Some(*boxed)
    }

    #[track_caller]
    pub fn get<'a, T: Any>(&'a self) -> Option<Ref<'a, T>> {
        let refcell = self.inner.get(&TypeId::of::<T>())?;
        let downcasted = Ref::map(refcell.borrow(), |any| {
            any.downcast_ref::<T>()
                .expect("downcasting to T should be valid for key TypeId::of::<T>")
        });
        Some(downcasted)
    }

    #[track_caller]
    pub fn get_mut<'a, T: Any>(&'a self) -> Option<RefMut<'a, T>> {
        let refcell = self.inner.get(&TypeId::of::<T>())?;
        let downcasted = RefMut::map(refcell.borrow_mut(), |any| {
            any.downcast_mut::<T>()
                .expect("downcasting to T should be valid for key TypeId::of::<T>")
        });
        Some(downcasted)
    }
}
