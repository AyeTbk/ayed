use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub type Ref<T> = Rc<RefCell<T>>;
pub type WeakRef<T> = Weak<RefCell<T>>;
