use std::path::{Path};

pub trait PathExt {
    fn to_str_or_err(&self) -> Result<&str, String>;
}

impl PathExt for Path {
    fn to_str_or_err(&self) -> Result<&str, String> {
        self.to_str().ok_or_else(|| format!("path '{self:?}' cannot be represented as Rust string"))
    }
}
