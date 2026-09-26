use std::collections::HashMap;

use crate::selection::Selections;

#[derive(Default)]
pub struct JumpPoints {
    // Manually saved jump points.
    pub registers: HashMap<char, (String, Selections)>,
    // Jump history from 'context switching' motions (ex: goto def, goto line).
    pub stack: Vec<(String, Selections)>,
    pub stack_cursor: Option<usize>,
}
