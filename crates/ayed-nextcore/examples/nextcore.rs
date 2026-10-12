use ayed_nextcore::Storage;

pub fn main() {
    let mut st = Storage::default();
    st.insert(77u32);

    let num1 = st.get::<u32>().unwrap();
    let num2 = st.get::<u32>().unwrap();
    let num3 = st.get::<u32>().unwrap();

    dbg!(format!("{num1} - {num2} - {num3}"));

    let mut num4 = st.get_mut::<u32>().unwrap();

    *num4 = 776;
    dbg!(format!("-- {num4}"));
}
