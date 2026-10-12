use ayed_nextcore::Storage;

pub fn main() {
    let mut bm = Storage::default();

    let (st, nm) = bm.get::<(Storage, u32)>();
}
