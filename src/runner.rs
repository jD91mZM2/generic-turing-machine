use crate::{
    expander::{Expanded, FnBody},
    parser::Move,
    FINISH_STATE
};

#[derive(Clone, Copy, Debug)]
pub enum Status {
    Progress,
    Accept,
    Reject
}

pub struct Runner {
    pub expanded: Expanded,
    pub next_state: String,
    pub head: Vec<Option<u8>>,
    pub tail: Vec<Option<u8>>,
    pub i: isize
}
impl Runner {
    pub fn new(expanded: Expanded, mut input: Vec<Option<u8>>) -> Self {
        let start = expanded.start.clone();
        if input.is_empty() {
            input.push(None);
        }
        Self {
            expanded,
            next_state: start,
            head: Vec::new(),
            tail: input,
            i: 0
        }
    }
    pub fn buffer<'a>(&'a mut self) -> impl Iterator<Item = Option<u8>> + 'a {
        self.head.iter().rev().chain(&self.tail).cloned()
    }
    pub fn value(&self) -> Option<u8> {
        if self.i >= 0 {
            self.tail[self.i as usize]
        } else {
            self.head[(-self.i as usize) - 1]
        }
    }
    pub fn value_mut(&mut self) -> &mut Option<u8> {
        if self.i >= 0 {
            &mut self.tail[self.i as usize]
        } else {
            &mut self.head[(-self.i as usize) - 1]
        }
    }
    pub fn next_fn(&self) -> Option<&FnBody> {
        let next_input = self.value();
        self.expanded.functions.iter()
                .find(|(key, _)|
                    key.match_state == self.next_state &&
                    key.match_input == next_input)
                .map(|(_, val)| val.as_ref().unwrap())
    }
    pub fn step(&mut self) -> Status {
        if self.next_state == FINISH_STATE {
            return Status::Accept;
        }

        let f = match self.next_fn() {
            Some(f) => f.clone(),
            None => return Status::Reject
        };

        *self.value_mut() = f.do_write;
        self.next_state = f.do_state;

        match f.do_move {
            Move::Current => (),
            Move::Next => {
                self.i += 1;
                if self.i >= self.tail.len() as isize {
                    self.tail.push(None);
                }
            },
            Move::Prev => {
                self.i -= 1;
                if -self.i - 1 >= self.head.len() as isize {
                    self.head.push(None);
                }
            }
        }
        Status::Progress
    }
}
