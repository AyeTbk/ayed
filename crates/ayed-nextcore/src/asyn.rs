use std::sync::{Arc, atomic::AtomicBool};

pub struct Asyn {
    executor: (),
    thread: (),
    shutdown_flag: Arc<AtomicBool>,
    async_done_flag: bool,
}

impl Asyn {
    pub fn spawn(&self) -> AsynTask {
        todo!()
    }
}

pub struct AsynTask {
    inner: (),
}

impl AsynTask {
    pub fn is_finished(&self) -> bool {
        todo!()
    }
}
