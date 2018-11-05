use crate::{
    expander::Expanded,
    types::Move
};

pub fn main(expanded: Expanded) {
    println!("name: generated turing machine");
    println!("init: {}", expanded.start);
    println!("accept: finish");
    for (f, body) in expanded.functions {
        let body = body.unwrap();
        println!(
            "\n{},{}\n{},{},{}",
            f.match_state,
            f.match_input.unwrap_or(b'_') as char,
            body.do_state,
            body.do_write.unwrap_or(b'_') as char,
            match body.do_move {
                Move::Current => "-",
                Move::Next => ">",
                Move::Prev => "<"
            }
        );
    }
}
