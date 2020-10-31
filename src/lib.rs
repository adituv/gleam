mod ast;
mod bit_string;
mod build;
mod cli;
mod config;
mod diagnostic;
mod docs;
mod erl;
mod error;
mod eunit;
mod format;
mod fs;
mod new;
mod parser;
mod pretty;
mod project;
mod shell;
mod typ;
mod warning;

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