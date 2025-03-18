#[derive(Default)]
pub struct Register {
    pub content: String,
    pub extra_content: Vec<String>,
}

impl Register {
    pub fn iter(&self) -> impl Iterator<Item = &str> + Clone {
        Some(self.content.as_str())
            .into_iter()
            .chain(self.extra_content.iter().map(|s| s.as_str()))
    }
}
