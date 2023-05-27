use crate::ast;

pub struct Automaton {
    nodes: Vec<Node>,
    start: NodeId,
}

pub type NodeId = usize;

pub struct Node {
    next: Vec<Connection>,
}

impl Node {
    pub fn is_end(&self) -> bool {
        self.next.is_empty()
    }
}

pub struct Connection {
    kind: ConnectionKind,
    to: NodeId,
}

pub enum ConnectionKind {
    Char(char),
    Any,
}

pub fn build_nfa(pattern: &str) -> Result<(), String> {
    let ast = ast::parse(pattern)?;
    todo!()
}
