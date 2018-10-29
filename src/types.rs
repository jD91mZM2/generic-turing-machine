use arenatree::NodeId;
use super::parser::*;

pub trait TypedNode<'a> where Self: Sized {
    fn cast(arena: &'a Arena, from: NodeId) -> Option<Self>;
    fn arena(&self) -> &Arena;
    fn node(&self) -> &ASTNode;
}

macro_rules! impl_types {
    ($($name:ident ($pat:pat) { $($block:tt)+ }),*) => {
        $(
        pub struct $name<'a>(&'a Arena, NodeId);
        impl<'a> TypedNode<'a> for $name<'a> {
            fn cast(arena: &'a Arena, from: NodeId) -> Option<Self> {
                match arena[from].kind {
                    $pat => Some($name(arena, from)),
                    _ => None
                }
            }
            fn arena(&self) -> &Arena {
                &self.0
            }
            fn node(&self) -> &ASTNode {
                &self.0[self.1]
            }
        }
        impl<'a> $name<'a> { $($block)+ }
        )*
    }
}
macro_rules! nth {
    ($self:expr; ($kind:ident) $n:expr) => {{
        let arena = $self.arena();
        $self.node().children(arena).nth($n)
            .and_then(|node| $kind::cast(arena, node))
            .expect("invalid ast")
    }}
}

impl_types! {
    Ident (ASTKind::Ident(_)) {
        pub fn name(&self) -> &str {
            match self.node().kind {
                ASTKind::Ident(ref name) => &name,
                _ => unreachable!()
            }
        }
        pub fn generics(&'a self) -> impl Iterator<Item = Ident> {
            let arena = self.arena();
            self.node().children(arena)
                .map(move |node| Ident::cast(arena, node).expect("invalid ast"))
        }
    },
    Char (ASTKind::Char(_)) {
        pub fn value(&self) -> Option<u8> {
            match self.node().kind {
                ASTKind::Char(c) => c,
                _ => unreachable!()
            }
        }
    },
    Movement (ASTKind::Move(_)) {
        pub fn operation(&self) -> Move {
            match self.node().kind {
                ASTKind::Move(m) => m,
                _ => unreachable!()
            }
        }
    },
    SetStart (ASTKind::SetStart) {
        pub fn target(&self) -> Ident {
            nth!(self; (Ident) 0)
        }
    },
    Function (ASTKind::Function) {
        pub fn match_state(&self) -> Ident {
            nth!(self; (Ident) 0)
        }
        pub fn match_input(&self) -> Char {
            nth!(self; (Char) 1)
        }
        pub fn do_write(&self) -> Char {
            nth!(self; (Char) 2)
        }
        pub fn do_state(&self) -> Ident {
            nth!(self; (Ident) 3)
        }
        pub fn do_move(&self) -> Movement {
            nth!(self; (Movement) 4)
        }
    },
    Root (ASTKind::Root) {
        pub fn start_assignments(&'a self) -> impl Iterator<Item = SetStart> {
            let arena = self.arena();
            self.node().children(arena)
                .filter_map(move |node| SetStart::cast(arena, node))
        }
        pub fn functions(&'a self) -> impl Iterator<Item = Function> {
            let arena = self.arena();
            self.node().children(arena)
                .filter_map(move |node| Function::cast(arena, node))
        }
    }
}
