use std::collections::BTreeMap;

use crate::ast::{self, Quantifier};

pub type NodeId = usize;
pub type ConnectionId = usize;
pub type CaptureGroupId = usize;

#[derive(Debug)]
pub struct Automaton {
    nodes: Vec<Node>,
    connections: Vec<Connection>,
    start: NodeId,
}

#[derive(Debug)]
pub struct Node {
    next: Vec<ConnectionId>,
    is_really_end: bool,
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
    pub fn needs_char(&self) -> bool {
        match &self.kind {
            ConnectionKind::AnyChar => true,
            ConnectionKind::Char(_) => true,
            _ => false,
        }
    }

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
    Direct,
    Subautomaton {
        start: NodeId,
        kind: SubautomatonKind,
    },
}

#[derive(Debug)]
pub enum SubautomatonKind {
    Repeating {
        repeat_min: u16,
        repeat_max: Option<u16>,
    },
    Capturing {
        capturing_group_idx: u16,
    },
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
        let end_node = self.build_node(ast_root, root_node);
        self.nodes[end_node as usize].is_really_end = true;
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
                let mut start_node = previous_node;
                let end_node = self.create_node();

                for _ in 0..*min {
                    let midway_node = self.build_node(ast_node, start_node);
                    start_node = midway_node;
                }
                if *min == 0 {
                    self.connect_nodes(start_node, end_node, ConnectionKind::Direct);
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
                            self.connect_nodes(post_start_node, end_node, ConnectionKind::Direct);
                        }
                    } else {
                        self.connect_nodes(start_node, end_node, ConnectionKind::Direct);
                    }
                } else {
                    let midway_node = self.build_node(ast_node, start_node);
                    self.connect_nodes(midway_node, end_node, ConnectionKind::Direct);
                    self.connect_nodes(midway_node, start_node, ConnectionKind::Direct);
                }
                end_node
            }
            Group(ast::Group {
                node: ast_node,
                capturing,
                name,
            }) => {
                // if *capturing {
                //     let capturing_group_idx = self.capture_group_count;
                //     self.capture_group_count += 1;

                //     let current_node = self.create_node();
                //     let start = self.build_nfa(ast_node);
                //     self.connect_nodes(
                //         previous_node,
                //         current_node,
                //         ConnectionKind::Subautomaton {
                //             start,
                //             kind: SubautomatonKind::Capturing {
                //                 capturing_group_idx,
                //             },
                //         },
                //     );
                //     current_node
                // } else {
                self.build_node(ast_node, previous_node)
                // }
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
        self.nodes.push(Node {
            next: vec![],
            is_really_end: false,
        });
        id
    }

    fn create_connection(&mut self, connection: Connection) -> ConnectionId {
        let id = self.connections.len();
        self.connections.push(connection);
        id
    }
}

pub fn run_nfa<I>(a: &Automaton, haystack: &I) -> bool
where
    I: Iterator<Item = char> + Clone,
{
    run_node(a, a.start, haystack).is_ok()
}

fn run_node<I>(a: &Automaton, node_id: NodeId, haystack: &I) -> Result<RunNodeSuccess<I>, ()>
where
    I: Iterator<Item = char> + Clone,
{
    let node = &a.nodes[node_id];
    if node.is_end() {
        assert!(node.is_really_end);
        return Ok(RunNodeSuccess {
            remaining_haystack: haystack.clone(),
            captures: Default::default(),
        });
    }

    'conn: for &connection_id in &node.next {
        let connection = &a.connections[connection_id];
        if connection.needs_char() {
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
        } else if let ConnectionKind::Direct = connection.kind {
            if let Ok(success) = run_node(a, connection.to, haystack) {
                return Ok(success);
            } else {
                continue 'conn;
            }
        }
    }

    Err(())
}

struct RunNodeSuccess<T> {
    remaining_haystack: T,
    captures: BTreeMap<CaptureGroupId, Capture>,
}

struct Capture {
    start_idx: usize,
    end_idx: usize,
}
