#[macro_use] extern crate clap;
#[macro_use] extern crate failure;

use clap::{AppSettings, Arg, SubCommand};
use rowan::TextRange;
use std::{fs, io::{self, prelude::*}};

mod expander;
mod parser;
mod runner;
mod subcommands;
mod tokenizer;
mod types;

use self::expander::Expander;
use self::parser::{Node, Types, Parser};
use self::tokenizer::{Token, Tokenizer};

pub const FINISH_STATE: &str = "finish";

fn main() -> io::Result<()> {
    let matches = app_from_crate!()
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("file")
            .help("Specifies the input file, defaults to STDIN"))
        .subcommand(SubCommand::with_name("generate")
            .about("Generates input to https://turingmachinesimulator.com/"))
        .subcommand(SubCommand::with_name("interactive")
            .about("Runs the turing machine in an interactive debugger"))
        .get_matches();

    let code = match matches.value_of("file") {
        None => {
            let mut code = String::new();
            io::stdin().read_to_string(&mut code)?;
            code
        },
        Some(path) => fs::read_to_string(path)?
    };
    let ast = Parser::new(Tokenizer::new(&code)).parse();
    let ast = ast.borrowed();

    let mut error = print_errors(&code, ast);
    if !ast.root_data().is_empty() {
        error = true;
        for error in ast.root_data() {
            eprintln!("error: {}", error);
        }
    }
    if error {
        return Ok(());
    }

    let expanded = match Expander::expand(ast) {
        Ok(functions) => functions,
        Err((span, err)) => {
            if let Some(span) = span {
                print_span(&code, span);
            }
            eprintln!("-> failed to expand: {}", err);
            return Ok(());
        }
    };

    for &span in &expanded.unreachable {
        print_span(&code, span);
        eprintln!("-> warning: unreachable code path");
    }

    match matches.subcommand_name() {
        Some("generate") => subcommands::generate::main(expanded),
        Some("interactive") => subcommands::interactive::interactive(&code, expanded),
        _ => unreachable!()
    }

    Ok(())
}
pub fn print_errors<R: rowan::TreeRoot<Types>>(code: &str, node: Node<R>) -> bool {
    let mut fail = false;
    if node.kind() == Token::Error {
        print_span(code, node.range());
        eprintln!("-> error: unexpected tokens");
        fail = true;
    }
    for child in node.children() {
        fail = print_errors(code, child) || fail;
    }
    fail
}
pub fn print_span(code: &str, span: TextRange) {
    let start = span.start().to_usize();
    let end   = span.end().to_usize();

    let mut ln = code[..end].lines().count();
    let llen = {
        let mut len = 1;
        let mut ln = ln + code[end..].lines().skip(1).take(1).count();
        while ln >= 10 {
            ln /= 10;
            len += 1;
        }
        len
    };

    let mut s = Some(start);
    for _ in 0..2 {
        s = s.and_then(|s| code[..s].rfind('\n'));
        if s.is_some() {
            ln -= 1;
        }
    }
    let mut s = s.map(|i| i + 1).unwrap_or(0);

    let mut prev = s;
    while prev < end {
        let next = s + code[s..].find('\n').map(|i| i + 1).unwrap_or(code.len() + 1 - s);

        eprintln!("{:>len$} {}", ln, &code[s..next-1], len=llen);

        if (start >= s && start < next) || (s >= start && end > s) {
            let col_start = start.saturating_sub(s);
            let col_end = (end - start).min(next-1 - s - col_start);
            eprintln!("{:start$}{}", "", "^".repeat(col_end), start = llen + 1 + col_start);
        }

        prev = s;
        s = next;
        ln += 1;
    }
}
