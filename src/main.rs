#[macro_use] extern crate clap;
#[macro_use] extern crate failure;

use clap::{AppSettings, Arg, SubCommand};

mod expander;
mod parser;
mod runner;
mod subcommands;
mod tokenizer;
mod types;

use self::expander::Expander;
use self::parser::Parser;
use self::tokenizer::{Span, Tokenizer};

use std::{fs, io::{self, prelude::*}};

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
    let tokens = match Tokenizer::new(code.chars()).collect::<Result<Vec<_>, _>>() {
        Ok(tokens) => tokens,
        Err((span, err)) => {
            if let Some(span) = span {
                print_span(&code, span);
            }
            eprintln!("-> failed to tokenize: {}", err);
            return Ok(());
        }
    };
    let ast = match Parser::new(tokens).parse() {
        Ok(None) => return Ok(()),
        Ok(Some(ast)) => ast,
        Err((span, err)) => {
            if let Some(span) = span {
                print_span(&code, span);
            }
            eprintln!("-> failed to parse: {}", err);
            return Ok(());
        }
    };
    let expanded = match Expander::expand(&ast) {
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
pub fn print_span(code: &str, span: Span) {
    let start = span.start as usize;
    let end   = span.end.map(|i| i as usize).unwrap_or(start + 1);

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
    }
    let mut s = s.map(|i| i + 1).unwrap_or(0);
    ln -= 1;

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
