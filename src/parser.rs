use arenatree::*;
use std::{fmt, iter::Peekable};
use super::{
    tokenizer::{Span, Token},
    FINISH_STATE
};

#[derive(Debug, Fail, PartialEq, Eq)]
pub enum ParseError {
    #[fail(display = "expected {:?}, found {:?}", _0, _1)]
    Expected(Token, Token),
    #[fail(display = "expected token of type {}, found {:?}", _0, _1)]
    ExpectedType(&'static str, Token),
    #[fail(display = "trailing input, expected end of line")]
    Trailing,
    #[fail(display = "re-definition of reserved state {:?}", _0)]
    Reserved(&'static str),
    #[fail(display = "unexpected eof")]
    UnexpectedEofGeneric,
    #[fail(display = "expected {:?}, found eof", _0)]
    UnexpectedEof(Token),
    #[fail(display = "expected token of type {}, found eof", _0)]
    UnexpectedEofType(&'static str),
    #[fail(display = "unexpected token {:?} not available in this position", _0)]
    Unexpected(Token)
}

pub type Arena = arenatree::Arena<'static, ASTNode>;
type Result<T> = std::result::Result<T, (Option<Span>, ParseError)>;

pub struct AST {
    pub arena: Arena,
    pub root: NodeId
}
impl fmt::Debug for AST {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.arena[self.root].format(f, &self.arena, 0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Current,
    Next,
    Prev
}
#[derive(Debug, PartialEq, Eq)]
pub enum ASTKind {
    Root,
    SetStart,
    Function,
    Move(Move),
    Ident(String),
    Char(Option<u8>)
}
pub struct ASTNode {
    pub span: Span,
    pub kind: ASTKind,
    pub node: Node,
}
impl AsRef<Node> for ASTNode {
    fn as_ref(&self) -> &Node { &self.node }
}
impl AsMut<Node> for ASTNode {
    fn as_mut(&mut self) -> &mut Node { &mut self.node }
}
impl ASTNode {
    pub fn children<'a>(&'a self, arena: &'a Arena) -> NodeIter<'a, ASTNode> {
        NodeIter {
            arena,
            cursor: self.node.child
        }
    }
    fn format(&self, f: &mut fmt::Formatter, arena: &Arena, indent: usize) -> fmt::Result {
        writeln!(f, "{:indent$}{:?}", "", self.kind, indent = indent)?;
        for child in self.children(arena) {
            arena[child].format(f, arena, indent+2)?;
        }
        Ok(())
    }
}

pub struct Parser<I>
    where I: Iterator<Item = (Span, Token)>
{
    arena: Arena,
    iter: Peekable<I>
}
impl<I> Parser<I>
    where I: Iterator<Item = (Span, Token)>
{
    pub fn new<T>(iter: T) -> Self
        where T: IntoIterator<Item = I::Item, IntoIter = I>
    {
        Self {
            arena: Arena::new(),
            iter: iter.into_iter().peekable()
        }
    }

    fn insert(&mut self, node: ASTNode) -> NodeId {
        self.arena.insert(node)
    }
    fn chain(&mut self, nodes: &[NodeId]) -> NodeId {
        let mut list = NodeList::new();
        list.push_all(nodes, &mut self.arena);
        list.node().expect("chain called on empty list")
    }

    fn peek(&mut self) -> Option<&Token> {
        self.iter.peek().map(|(_, t)| t)
    }
    fn next(&mut self) -> Result<(Span, Token)> {
        self.iter.next().ok_or((None, ParseError::UnexpectedEofGeneric))
    }
    fn expect(&mut self, expected: Token) -> Result<Span> {
        match self.next() {
            Err(_) => Err((None, ParseError::UnexpectedEof(expected))),
            Ok((span, actual)) => if expected == actual {
                Ok(span)
            } else {
                Err((Some(span), ParseError::Expected(expected, actual)))
            }
        }
    }
    fn whitespace(&mut self) {
        while self.peek().map(|t| *t == Token::Separator).unwrap_or(false) {
            self.next().unwrap();
        }
    }
    fn parse_ident(&mut self) -> Result<ASTNode> {
        let (mut span, name) = match self.next()? {
            (span, Token::Ident(name)) => (span, name),
            (span, token) => return Err((Some(span), ParseError::ExpectedType("ident", token)))
        };

        let mut children = NodeList::new();
        if self.peek() == Some(&Token::AngleBOpen) {
            loop {
                self.next().unwrap();
                self.whitespace();

                let ident = self.parse_ident()?;
                let ident = self.insert(ident);
                children.push(ident, &mut self.arena);

                if self.peek() == Some(&Token::Comma) {
                    continue;
                }
                let end = self.expect(Token::AngleBClose)?;
                span = span.until(end);
                break;
            }
        }

        Ok(ASTNode {
            kind: ASTKind::Ident(name),
            span,
            node: Node::with_child(children.node())
        })
    }
    fn parse_char(&mut self) -> Result<ASTNode> {
        match self.next() {
            Err(_) => Err((None, ParseError::UnexpectedEofType("char"))),
            Ok((span, Token::Char(num))) => Ok(ASTNode {
                kind: ASTKind::Char(num),
                span,
                node: Node::with_child(None)
            }),
            Ok((span, token)) => Err((Some(span), ParseError::ExpectedType("char", token))),
        }
    }
    fn parse_next(&mut self) -> Result<ASTNode> {
        match self.peek().ok_or((None, ParseError::UnexpectedEofGeneric))? {
            Token::Start => {
                let (start, _) = self.next().unwrap();
                self.expect(Token::Equal)?;
                let ident = self.parse_ident()?;
                let end = ident.span;
                let ident = self.insert(ident);

                Ok(ASTNode {
                    kind: ASTKind::SetStart,
                    span: start.until(end),
                    node: Node::with_child(ident)
                })
            },
            Token::Ident(ref s) if s == FINISH_STATE => {
                let (span, _) = self.next().unwrap();
                Err((Some(span), ParseError::Reserved(FINISH_STATE)))
            },
            Token::Ident(_) => {
                let i_state = self.parse_ident()?;
                let start = i_state.span;
                let i_char = self.parse_char()?;
                self.expect(Token::Equal)?;
                self.whitespace();
                let o_state = self.parse_char()?;
                self.expect(Token::Semicolon)?;
                let o_char = self.parse_ident()?;

                let (end, movement) = match self.next() {
                    Err(_) => return Err((None, ParseError::UnexpectedEofType("movement"))),
                    Ok((span, Token::Current)) => (span, Move::Current),
                    Ok((span, Token::Next)) => (span, Move::Next),
                    Ok((span, Token::Prev)) => (span, Move::Prev),
                    Ok((span, token)) => return Err((Some(span), ParseError::ExpectedType("movement", token)))
                };
                let movement = ASTNode {
                    kind: ASTKind::Move(movement),
                    span: end,
                    node: Node::with_child(None)
                };

                let i_state  = self.insert(i_state);
                let i_char   = self.insert(i_char);
                let o_state  = self.insert(o_state);
                let o_char   = self.insert(o_char);
                let movement = self.insert(movement);

                let children = self.chain(&[
                    i_state,
                    i_char,
                    o_state,
                    o_char,
                    movement
                ]);

                Ok(ASTNode {
                    kind: ASTKind::Function,
                    span: start.until(end),
                    node: Node::with_child(children)
                })
            },
            _ => {
                let (span, token) = self.next().unwrap();
                Err((Some(span), ParseError::Unexpected(token)))
            }
        }
    }
    pub fn parse(mut self) -> Result<Option<AST>> {
        let mut children = NodeList::new();
        let mut start = None;
        let mut end = None;
        loop {
            self.whitespace();

            if self.peek().is_none() {
                break;
            }

            let node = self.parse_next()?;

            end = Some(node.span);
            start = start.or(end);

            let node = self.insert(node);
            children.push(node, &mut self.arena);

            match self.peek() {
                None | Some(Token::Separator) => (),
                Some(_) => {
                    let (span, _) = self.next().unwrap();
                    return Err((Some(span), ParseError::Trailing));
                }
            }
        }

        assert!(start.is_none() == end.is_none());
        if start.is_none() {
            return Ok(None);
        }

        let root = ASTNode {
            kind: ASTKind::Root,
            span: start.unwrap().until(end.unwrap()),
            node: Node::with_child(children.node())
        };
        let root = self.insert(root);

        Ok(Some(AST {
            arena: self.arena,
            root
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::tokenizer::{Span, Token, Tokenizer};
    use super::*;

    fn parse(tokens: Vec<Token>) -> String {
        let tokens = tokens.into_iter().map(|t| (Span::default(), t));
        let ast = Parser::new(tokens).parse()
            .expect("failed to parse ast")
            .expect("empty ast");
        format!("{:?}", ast)
    }

    #[test]
    fn basic() {
        assert_eq!(
            parse(vec![
                Token::Ident("invert".into()),
                Token::Char(Some(0)),
                Token::Equal,
                Token::Char(Some(1)),
                Token::Semicolon,
                Token::Ident("invert".into()),
                Token::Next,
            ]),
r#"Root
  Function
    Ident("invert")
    Char(Some(0))
    Char(Some(1))
    Ident("invert")
    Move(Next)
"#
        );
        assert_eq!(
            parse(vec![
                Token::Start,
                Token::Equal,
                Token::Ident("invert".into()),
                Token::Separator,
                Token::Ident("invert".into()),
                Token::Char(Some(1)),
                Token::Equal,
                Token::Char(Some(0)),
                Token::Semicolon,
                Token::Ident("finish".into()),
                Token::Current,
            ]),
r#"Root
  SetStart
    Ident("invert")
  Function
    Ident("invert")
    Char(Some(1))
    Char(Some(0))
    Ident("finish")
    Move(Current)
"#
        );
    }
    #[test]
    fn generic() {
        assert_eq!(
            parse(vec![
                Token::Ident("back".into()),
                Token::AngleBOpen,
                Token::Ident("fn".into()),
                Token::AngleBClose,
                Token::Char(None),
                Token::Equal,
                Token::Char(None),
                Token::Semicolon,
                Token::Ident("back".into()),
                Token::AngleBOpen,
                Token::Ident("fn".into()),
                Token::AngleBClose,
                Token::Prev
            ]),
r#"Root
  Function
    Ident("back")
      Ident("fn")
    Char(None)
    Char(None)
    Ident("back")
      Ident("fn")
    Move(Prev)
"#
        );
        assert_eq!(
            parse(vec![
                Token::Ident("add".into()),
                Token::AngleBOpen,
                Token::Ident("x".into()),
                Token::Comma,
                Token::Ident("y".into()),
                Token::AngleBClose,
                Token::Char(None),
                Token::Equal,
                Token::Char(None),
                Token::Semicolon,
                Token::Ident("finish".into()),
                Token::Current
            ]),
r#"Root
  Function
    Ident("add")
      Ident("x")
      Ident("y")
    Char(None)
    Char(None)
    Ident("finish")
    Move(Current)
"#
        );
        assert_eq!(
            parse(vec![
                Token::Ident("thing".into()),
                Token::Char(None),
                Token::Equal,
                Token::Char(None),
                Token::Semicolon,
                Token::Ident("add".into()),
                Token::AngleBOpen,
                Token::Ident("num".into()),
                Token::Comma,
                Token::Ident("sub".into()),
                Token::AngleBOpen,
                Token::Ident("num".into()),
                Token::Comma,
                Token::Ident("num".into()),
                Token::AngleBClose,
                Token::AngleBClose,
                Token::Current
            ]),
r#"Root
  Function
    Ident("thing")
    Char(None)
    Char(None)
    Ident("add")
      Ident("num")
      Ident("sub")
        Ident("num")
        Ident("num")
    Move(Current)
"#
        );
    }
    #[test]
    fn spans() {
        let tokens =
            Tokenizer::new("add<x, y> _ = _; finish current".chars())
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("failed to tokenize");
        let ast = Parser::new(tokens).parse().expect("failed to parse").expect("empty ast");

        assert_eq!(
            ast.arena.into_inner()
                .into_iter()
                .filter_map(|node| node)
                .map(|node| (node.span, node.kind)).collect::<Vec<_>>(),
            vec![
                (Span { start: 4, end: Some(5) }, ASTKind::Ident("x".into())),
                (Span { start: 7, end: Some(8) }, ASTKind::Ident("y".into())),
                (Span { start: 0, end: Some(9) }, ASTKind::Ident("add".into())),
                (Span { start: 10, end: Some(11) }, ASTKind::Char(None)),
                (Span { start: 14, end: Some(15) }, ASTKind::Char(None)),
                (Span { start: 17, end: Some(23) }, ASTKind::Ident("finish".into())),
                (Span { start: 24, end: Some(31) }, ASTKind::Move(Move::Current)),
                (Span { start: 0, end: Some(31) }, ASTKind::Function),
                (Span { start: 0, end: Some(31) }, ASTKind::Root),
            ]
        );
    }
}
