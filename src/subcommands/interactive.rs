use crate::{
    expander::Expanded,
    runner::{Runner, Status}
};
use rustyline::{error::ReadlineError, Editor};
use std::collections::HashMap;

pub fn interactive(code: &str, expanded: Expanded) {
    if let Some(mut app) = Interactive::new(code, expanded) {
        app.main();
    }
}

struct Interactive<'a> {
    breakpoints: HashMap<usize, u32>,
    breakpoint_id: usize,
    code: &'a str,
    editor: Editor<()>,
    runner: Runner
}
impl<'a> Interactive<'a> {
    pub fn new(code: &'a str, expanded: Expanded) -> Option<Self> {
        println!("Welcome to an interactive turing machine runner!");
        println!("To set a breakpoint, type `breakpoint` and the line number.");
        println!("To clear breakpoints, type `clear` and optionally specify a number.");
        println!("To run the machine until the next breakpoint, type `run`.");
        println!("To step the machine, type `step` or `next`.");

        let mut editor = Editor::<()>::new();
        println!();
        println!("Please enter the input tape, and use space as empty slot");
        let input = match editor.readline("Input: ") {
            Ok(input) => input,
            Err(ReadlineError::Eof) | Err(ReadlineError::Interrupted) => return None,
            Err(err) => {
                eprintln!("readline error: {}", err);
                return None;
            }
        };
        let input = input.bytes().map(|c| Some(c).filter(|&c| c != b' ')).collect();

        Some(Self {
            breakpoints: HashMap::new(),
            breakpoint_id: 1,
            code,
            editor,
            runner: Runner::new(expanded, input)
        })
    }

    fn step(&mut self) -> bool {
        match self.runner.step() {
            Status::Progress => false,
            Status::Accept => { eprintln!("accepted: machine has entered the finish state"); true },
            Status::Reject => { eprintln!("rejected: no matching state handler"); true }
        }
    }
    fn print_location(&mut self) {
        if let Some(f) = self.runner.next_fn() {
            println!("Next state: {}", self.runner.next_state);
            let start = f.span.start as usize;
            let end = f.span.end.map(|i| i as usize)
                .unwrap_or_else(|| self.code[start..]
                    .find('\n')
                    .unwrap_or(self.code.len() - start));
            let line = 1 + &self.code[..start].lines().count();
            println!("{} {}", line, &self.code[start..end]);
        }

        let i = (self.runner.head.len() as isize + self.runner.i) as usize;
        let mut iter = self.runner.buffer()
            .skip(i.saturating_sub(5))
            .take(5 + i.min(5))
            .peekable();
        if iter.peek().is_some() {
            print!("Tape: {:leading$}", "", leading = 5usize.saturating_sub(i));
            for c in iter {
                print!("{}", c.unwrap_or(b' ') as char);
            }
            println!();
            println!("           ^");
        }
    }

    fn main(&mut self) {
        println!();
        self.print_location();

        let mut last = None;

        'main: loop {
            let line = match self.editor.readline("> ") {
                Ok(ref line) if line.is_empty() => last.unwrap_or_else(String::new),
                Ok(line) => {
                    self.editor.add_history_entry(line.clone());
                    line
                },
                Err(ReadlineError::Eof) | Err(ReadlineError::Interrupted) => break,
                Err(err) => {
                    eprintln!("readline error: {}", err);
                    return;
                }
            };
            last = Some(line);

            let mut args = last.as_ref().unwrap().split_whitespace();
            let cmd = match args.next() {
                Some(cmd) => cmd,
                None => continue
            };

            match &*cmd {
                "breakpoint" | "b" => {
                    let line = match args.next().and_then(|arg| arg.parse().ok()) {
                        Some(line) => line,
                        None => {
                            eprintln!("breakpoint <line>");
                            continue;
                        }
                    };
                    let mut offset = 0;
                    for _ in 1..line {
                        offset += 1 + match self.code[offset..].find('\n') {
                            Some(br) => br,
                            None => {
                                eprintln!("invalid line");
                                continue 'main;
                            }
                        };
                    }
                    self.breakpoints.insert(self.breakpoint_id, offset as u32);
                    println!("Breakpoint #{} created!", self.breakpoint_id);
                    self.breakpoint_id += 1;
                },
                "clear" | "c" => {
                    if let Some(i) = args.next() {
                        let i = match i.parse() {
                            Ok(i) => i,
                            Err(_) => {
                                eprintln!("clear [number]");
                                continue;
                            }
                        };
                        if self.breakpoints.remove(&i).is_some() {
                            println!("Breakpoint #{} removed!", i);
                        } else {
                            eprintln!("No such breakpoint");
                        }
                    } else {
                        self.breakpoints.clear();
                        println!("All breakpoints cleared!");
                    }
                },
                "run" => {
                    'run: loop {
                        if self.step() {
                            break;
                        }
                        if let Some(f) = self.runner.next_fn() {
                            for (i, &b) in &self.breakpoints {
                                if b >= f.span.start && f.span.end.map(|end| b < end).unwrap_or(false) {
                                    println!("Breakpoint #{} reached", i);
                                    break 'run;
                                }
                            }
                        }
                    }
                    self.print_location();
                }
                "step" | "s" | "next" | "n" => {
                    self.step();
                    self.print_location();
                },
                _ => eprintln!("unknown command")
            }
        }
    }
}
