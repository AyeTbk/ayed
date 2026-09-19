use std::collections::HashMap;

use crate::selection::Selections;

#[derive(Default)]
pub struct JumpPoints {
    pub registers: HashMap<char, (String, Selections)>,
}
