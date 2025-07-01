#[derive(Default)]
pub struct Suggestions {
    pub items: Vec<String>,
    /// Selected item index, 1 based, where 0 means none.
    pub selected_item: i32,
    pub original_symbols: Vec<String>,
}
