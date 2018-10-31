use super::{
    parser::*,
    tokenizer::Token
};

use rowan::SmolStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Move {
    Current,
    Next,
    Prev,
}

pub trait TypedNode<R: rowan::TreeRoot<Types>> where Self: Sized {
    fn cast(node: Node<R>) -> Option<Self>;
    fn node(&self) -> &Node<R>;
    fn text<'a>(&'a self) -> Option<&'a str>
        where R: 'a
    {
        self.node().borrowed()
            .leaf_text()
            .map(SmolStr::as_str)
    }
}

macro_rules! impl_types {
    ($($name:ident ($pat:pat) { $($block:tt)* }),*) => {
        $(
        pub struct $name<R: rowan::TreeRoot<Types>>(Node<R>);
        impl<R: rowan::TreeRoot<Types>> TypedNode<R> for $name<R> {
            fn cast(from: Node<R>) -> Option<Self> {
                match from.kind() {
                    $pat => Some($name(from)),
                    _ => None
                }
            }
            fn node(&self) -> &Node<R> {
                &self.0
            }
        }
        impl<R: rowan::TreeRoot<Types>> $name<R> { $($block)* }
        )*
    }
}
macro_rules! nth {
    ($self:expr; ($kind:ident) $n:expr) => {{
        $self.node().children()
            .filter_map($kind::cast)
            .nth($n)
            .expect("invalid ast")
    }}
}

impl_types! {
    Ident (Token::Ident) {
        pub fn as_str(&self) -> &str {
            self.text().unwrap_or_default()
        }
    },
    Type (Token::Type) {
        pub fn name(&self) -> Ident<R> {
            nth!(self; (Ident) 0)
        }
        pub fn generics(&self) -> impl Iterator<Item = Type<R>> {
            self.node().children().filter_map(Type::cast)
        }
    },
    Char (Token::Char) {
        pub fn value(&self) -> Option<u8> {
            let s = self.text().unwrap();
            let mut s = s.chars();
            let c = s.next().unwrap();
            if c >= '0' && c <= '9' {
                Some(c as u8)
            } else if c == '\'' {
                let c = s.next().unwrap();
                assert_eq!(s.next(), Some('\''));
                assert!(c.is_ascii());
                Some(c as u8)
            } else if c == '_' {
                None
            } else {
                panic!("invalid ast");
            }
        }
    },
    Movement (Token::Move) {
        pub fn operation(&self) -> Move {
            match self.node().first_child().expect("invalid ast").kind() {
                Token::Current => Move::Current,
                Token::Next => Move::Next,
                Token::Prev => Move::Prev,
                _ => panic!("invalid ast")
            }
        }
    },
    SetStart (Token::SetStart) {
        pub fn target(&self) -> Type<R> {
            nth!(self; (Type) 0)
        }
    },
    Function (Token::Function) {
        pub fn match_state(&self) -> Type<R> {
            nth!(self; (Type) 0)
        }
        pub fn match_input(&self) -> Char<R> {
            nth!(self; (Char) 0)
        }
        pub fn do_write(&self) -> Char<R> {
            nth!(self; (Char) 1)
        }
        pub fn do_state(&self) -> Type<R> {
            nth!(self; (Type) 1)
        }
        pub fn do_move(&self) -> Movement<R> {
            nth!(self; (Movement) 0)
        }
    },
    Root (Token::Root) {
        pub fn start_assignments(&self) -> impl Iterator<Item = SetStart<R>> {
            self.node().children().filter_map(SetStart::cast)
        }
        pub fn functions(&self) -> impl Iterator<Item = Function<R>> {
            self.node().children().filter_map(Function::cast)
        }
    }
}
