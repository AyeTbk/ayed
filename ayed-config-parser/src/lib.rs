pub mod ast;
use ast::Ast;

mod parser;
use parser::Parser;

mod token;

mod error;
pub use self::error::{Error, ErrorKind};

pub fn parse_module(src: &str) -> (Ast, Vec<Error>) {
    Parser::new(src).parse_module()
}
