use std::collections::BTreeMap;

use crate::ast::{self, Quantifier};

pub type NodeId = usize;
pub type ConnectionId = usize;
pub type CaptureGroupId = u16;

#[derive(Debug)]
pub struct Automaton {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    start: NodeId,
}

#[derive(Debug)]
pub struct Node {
    next: Vec<ConnectionId>,
}

impl Node {
    pub fn is_end(&self) -> bool {
        self.next.is_empty()
    }
}

#[derive(Debug)]
pub struct Connection {
    kind: ConnectionKind,
    to: NodeId,
}

impl Connection {
    pub fn matches_char(&self, chr: char) -> bool {
        match &self.kind {
            ConnectionKind::AnyChar => true,
            ConnectionKind::Char(c) if *c == chr => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum ConnectionKind {
    Char(char),
    AnyChar,
    Capture {
        start: bool,
        capture_group_idx: CaptureGroupId,
    },
    Direct,
}

#[derive(Debug)]
pub struct Repeat {
    pub min: u16,
    pub max: u16,
}

pub fn build_nfa(ast: &ast::Ast) -> Automaton {
    let mut builder = NfaBuilder::new();
    let start = builder.build_nfa(&ast.root);
    Automaton {
        start,
        nodes: builder.nodes,
        connections: builder.connections,
    }
}

struct NfaBuilder {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    capture_group_count: u16,
}

impl NfaBuilder {
    pub fn new() -> Self {
        Self {
            nodes: Default::default(),
            connections: Default::default(),
            capture_group_count: 0,
        }
    }

    pub fn build_nfa(&mut self, ast_root: &ast::Node) -> NodeId {
        let root_node = self.create_node();
        self.build_node(ast_root, root_node);
        root_node
    }

    fn build_node(&mut self, ast_node: &ast::Node, previous_node: NodeId) -> NodeId {
        use ast::Node::*;
        match ast_node {
            Nothing => {
                let current_node = self.create_node();

                self.connect_nodes(previous_node, current_node, ConnectionKind::Direct);
                current_node
            }
            Char(ch) => {
                let current_node = self.create_node();

                let connection_kind = if *ch == '.' {
                    ConnectionKind::AnyChar
                } else {
                    ConnectionKind::Char(*ch)
                };
                self.connect_nodes(previous_node, current_node, connection_kind);
                current_node
            }
            Concat(ast_nodes) => {
                let mut current_node = previous_node;
                for ast_node in ast_nodes {
                    current_node = self.build_node(ast_node, current_node);
                }
                current_node
            }
            Alteratives(ast_nodes) => {
                let current_node = self.create_node();
                for ast_node in ast_nodes {
                    let midway_node = self.build_node(ast_node, previous_node);
                    self.connect_nodes(midway_node, current_node, ConnectionKind::Direct);
                }
                current_node
            }
            Quantified {
                node: ast_node,
                quantifier: Quantifier { min, max, lazy },
            } => {
                // Connections to establish, given in such an order that matching is "lazy".
                let mut connections = vec![];

                let mut start_node = previous_node;
                let end_node = self.create_node();

                for _ in 0..*min {
                    let midway_node = self.build_node(ast_node, start_node);
                    start_node = midway_node;
                }
                if *min == 0 {
                    connections.push((start_node, end_node, ConnectionKind::Direct));
                }

                if let Some(max) = max {
                    let extra_repetitions = *max - *min;
                    if extra_repetitions > 0 {
                        for i in 0..extra_repetitions {
                            let mut post_start_node = start_node;
                            for _ in 0..(i + 1) {
                                let midway_node = self.build_node(ast_node, post_start_node);
                                post_start_node = midway_node;
                            }
                            connections.push((post_start_node, end_node, ConnectionKind::Direct));
                        }
                    } else {
                        connections.push((start_node, end_node, ConnectionKind::Direct));
                    }
                } else {
                    let midway_node = self.build_node(ast_node, start_node);
                    connections.push((midway_node, end_node, ConnectionKind::Direct));
                    connections.push((midway_node, start_node, ConnectionKind::Direct));
                }

                if !lazy {
                    connections.reverse();
                }

                // Establish connections
                for (node_a, node_b, conn_kind) in connections {
                    self.connect_nodes(node_a, node_b, conn_kind);
                }

                end_node
            }
            Group(ast::Group {
                node: ast_node,
                capturing,
                name,
            }) => {
                if *capturing {
                    let capture_group_idx = self.capture_group_count;
                    self.capture_group_count += 1;

                    let capture_start_node = self.create_node();
                    self.connect_nodes(
                        previous_node,
                        capture_start_node,
                        ConnectionKind::Capture {
                            start: true,
                            capture_group_idx,
                        },
                    );

                    let pattern_node = self.build_node(ast_node, capture_start_node);

                    let capture_end_node = self.create_node();
                    self.connect_nodes(
                        pattern_node,
                        capture_end_node,
                        ConnectionKind::Capture {
                            start: false,
                            capture_group_idx,
                        },
                    );

                    capture_end_node
                } else {
                    self.build_node(ast_node, previous_node)
                }
            }
        }
    }

    fn connect_nodes(&mut self, from: NodeId, to: NodeId, kind: ConnectionKind) {
        assert!(self.nodes.len() > from);
        assert!(self.nodes.len() > to);
        let connection_id = self.create_connection(Connection { to, kind });
        self.nodes[from].next.push(connection_id);
    }

    fn create_node(&mut self) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node { next: vec![] });
        id
    }

    fn create_connection(&mut self, connection: Connection) -> ConnectionId {
        let id = self.connections.len();
        self.connections.push(connection);
        id
    }
}

#[derive(Clone)]
struct Input<I> {
    idx: usize,
    haystack: I,
}

impl<I: Iterator<Item = char> + Clone> Input<I> {
    pub fn new(haystack: I) -> Self {
        Self { idx: 0, haystack }
    }

    pub fn next(&mut self) -> Option<char> {
        let next = self.haystack.next();
        if next.is_some() {
            self.idx += 1;
        }
        next
    }
}

pub fn run_nfa<I>(a: &Automaton, haystack: I) -> Result<RunNodeSuccess, ()>
where
    I: Iterator<Item = char> + Clone,
{
    run_node(a, a.start, &Input::new(haystack))
}

fn run_node<I>(a: &Automaton, node_id: NodeId, haystack: &Input<I>) -> Result<RunNodeSuccess, ()>
where
    I: Iterator<Item = char> + Clone,
{
    let node = &a.nodes[node_id];
    if node.is_end() {
        return Ok(RunNodeSuccess {
            captures: Default::default(),
        });
    }

    'conn: for &connection_id in &node.next {
        let connection = &a.connections[connection_id];
        match &connection.kind {
            ConnectionKind::Char(_) | ConnectionKind::AnyChar => {
                let mut h = haystack.clone();
                let maybe_chr = h.next();
                let chr = if let Some(chr) = maybe_chr {
                    chr
                } else {
                    // Reached the end of haystack but the node was not the end: failed to match
                    return Err(());
                };
                if connection.matches_char(chr) {
                    if let Ok(success) = run_node(a, connection.to, &h) {
                        return Ok(success);
                    } else {
                        continue 'conn;
                    }
                } else {
                    continue 'conn;
                }
            }
            &ConnectionKind::Capture {
                start,
                capture_group_idx,
            } => {
                if start {
                    if let Ok(mut success) = run_node(a, connection.to, haystack) {
                        let maybe_start_idx = &mut success
                            .captures
                            .entry(capture_group_idx)
                            .or_default()
                            .start_idx;
                        if maybe_start_idx.is_none() {
                            *maybe_start_idx = Some(haystack.idx);
                        }
                        return Ok(success);
                    } else {
                        continue 'conn;
                    }
                } else {
                    if let Ok(mut success) = run_node(a, connection.to, haystack) {
                        let maybe_end_idx = &mut success
                            .captures
                            .entry(capture_group_idx)
                            .or_default()
                            .end_idx;
                        if maybe_end_idx.is_none() {
                            *maybe_end_idx = Some(haystack.idx);
                        }
                        return Ok(success);
                    } else {
                        continue 'conn;
                    }
                }
            }
            ConnectionKind::Direct => {
                if let Ok(success) = run_node(a, connection.to, haystack) {
                    return Ok(success);
                } else {
                    continue 'conn;
                }
            }
        }
    }

    Err(())
}

pub struct RunNodeSuccess {
    captures: BTreeMap<CaptureGroupId, Capture>,
}

#[derive(Debug, Default)]
pub struct Capture {
    start_idx: Option<usize>,
    end_idx: Option<usize>,
}
