pub mod ast;
pub mod bit_string;
pub mod build;
pub mod cli;
pub mod config;
pub mod diagnostic;
pub mod docs;
pub mod erl;
pub mod error;
pub mod eunit;
pub mod format;
pub mod fs;
pub mod new;
pub mod parser;
pub mod pretty;
pub mod project;
pub mod shell;
pub mod typ;
pub mod warning;

lalrpop_mod!(
    #[allow(
        clippy::all,
        clippy::use_self,
        clippy::option_option,
        clippy::inefficient_to_string,
        dead_code,
        deprecated,
        unused_parens,
        unused_qualifications
    )]
    grammar
);

#[macro_use]
extern crate lalrpop_util;

#[macro_use]
extern crate im;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[macro_use]
extern crate lazy_static;