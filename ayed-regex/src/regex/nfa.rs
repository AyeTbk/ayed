use crate::ast;

pub type NodeId = usize;
pub type ConnectionId = usize;

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
    Subautomata {
        start: NodeId,
        repeat_min: u16,
        repeat_max: Option<u16>,
    },
}

#[derive(Debug)]
pub struct Repeat {
    pub min: u16,
    pub max: u16,
}

pub fn build_nfa(ast: &ast::Ast) -> Automaton {
    dbg!(ast);

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
}

impl NfaBuilder {
    pub fn new() -> Self {
        Self {
            nodes: Default::default(),
            connections: Default::default(),
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
                quantifier,
            } => {
                let current_node = self.create_node();
                let start = self.build_nfa(ast_node);
                self.connect_nodes(
                    previous_node,
                    current_node,
                    ConnectionKind::Subautomata {
                        start,
                        repeat_min: quantifier.min,
                        repeat_max: quantifier.max,
                    },
                );
                current_node
            }
            Group(ast_node) => self.build_node(ast_node, previous_node),
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
        return Ok(RunNodeSuccess {
            remaining_haystack: haystack.clone(),
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
                }
            }
        } else if let ConnectionKind::Subautomata {
            start,
            repeat_min,
            repeat_max,
        } = connection.kind
        {
            // Run node 'start' on repeat and it must return true at least 'repeat_min'
            // times to be successful.
            // Greedy by default so try to match as much as possible.
            if let Some(repeat_max) = repeat_max {
                assert!(repeat_min <= repeat_max); // TODO use type system to remove this sanity check?
            }
            let mut h = haystack.clone();
            let mut times_matched: i32 = 0;
            while repeat_max.is_none() || times_matched < repeat_max.unwrap().into() {
                if let Ok(success) = run_node(a, start, &h) {
                    times_matched += 1;
                    h = success.remaining_haystack;
                } else {
                    break;
                }
            }
            if times_matched >= repeat_min.into() {
                if let Ok(success) = run_node(a, connection.to, &h) {
                    return Ok(success);
                }
            } else {
                continue 'conn;
            }
        } else if let ConnectionKind::Direct = connection.kind {
            if let Ok(success) = run_node(a, connection.to, haystack) {
                return Ok(success);
            }
        }
    }

    Err(())
}

struct RunNodeSuccess<T> {
    remaining_haystack: T,
}
