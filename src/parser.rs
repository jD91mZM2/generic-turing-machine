use rowan::SmolStr;
use std::iter::Peekable;
use super::{
    tokenizer::Token,
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
    UnexpectedEof(Token)
}

pub struct Types;
impl rowan::Types for Types {
    type Kind = Token;
    type RootData = Vec<ParseError>;
}

pub type Node<R = rowan::OwnedRoot<Types>> = rowan::SyntaxNode<Types, R>;

pub struct Parser<I>
    where I: Iterator<Item = (Token, SmolStr)>
{
    iter: Peekable<I>,
    builder: rowan::GreenNodeBuilder<Types>,
    errors: Vec<ParseError>
}
impl<I> Parser<I>
    where I: Iterator<Item = (Token, SmolStr)>
{
    pub fn new<T>(iter: T) -> Self
        where T: IntoIterator<Item = I::Item, IntoIter = I>
    {
        Self {
            iter: iter.into_iter().peekable(),
            builder: rowan::GreenNodeBuilder::new(),
            errors: Vec::new()
        }
    }

    fn peek_str(&mut self) -> Option<&(Token, SmolStr)> {
        while self.iter.peek().map(|(t, _)| t.is_trivia()).unwrap_or(false) {
            self.bump();
        }
        self.iter.peek()
    }
    fn peek(&mut self) -> Option<Token> {
        self.peek_str().map(|&(t, _)| t)
    }
    fn bump(&mut self) {
        match self.iter.next() {
            Some((token, s)) => self.builder.leaf(token, s),
            None => self.errors.push(ParseError::UnexpectedEofGeneric)
        }
    }
    fn expect(&mut self, expected: Token) {
        match self.peek() {
            None => self.errors.push(ParseError::UnexpectedEof(expected)),
            Some(actual) => {
                if expected != actual {
                    self.builder.start_internal(Token::Error);
                    while { self.bump(); self.peek().map(|t| t != expected).unwrap_or(false) } {}
                    self.builder.finish_internal();
                }
                self.bump();
            }
        }
    }
    fn newlines(&mut self) {
        while self.peek().map(|t| t == Token::Separator || t.is_trivia()).unwrap_or(false) {
            self.bump();
        }
    }
    fn parse_ident(&mut self) {
        self.builder.start_internal(Token::Type);

        self.expect(Token::Ident);

        if self.peek() == Some(Token::AngleBOpen) {
            loop {
                self.bump();
                self.newlines();
                self.parse_ident();

                if self.peek() != Some(Token::Comma) {
                    self.expect(Token::AngleBClose);
                    break;
                }
            }
        }

        self.builder.finish_internal();
    }
    fn parse_next(&mut self) {
        match self.peek_str() {
            Some((Token::Start, _)) => {
                self.builder.start_internal(Token::SetStart);

                self.bump();
                self.expect(Token::Equal);
                self.parse_ident();

                self.builder.finish_internal();
            },
            Some((Token::Ident, ref s)) if s == FINISH_STATE => {
                self.builder.start_internal(Token::Error);
                self.bump();
                self.builder.finish_internal();
            },
            Some((Token::Ident, _)) => {
                self.builder.start_internal(Token::Function);

                self.parse_ident();
                self.expect(Token::Char);
                self.expect(Token::Equal);
                self.newlines();
                self.expect(Token::Char);
                self.expect(Token::Semicolon);
                self.parse_ident();

                if self.peek().map(|t| !t.is_move()).unwrap_or(false) {
                    self.builder.start_internal(Token::Error);
                    while { self.bump(); self.peek().map(|t| !t.is_move()).unwrap_or(false) } {}
                    self.builder.finish_internal();
                }

                self.builder.start_internal(Token::Move);
                self.bump();
                self.builder.finish_internal();

                self.builder.finish_internal();
            },
            None => self.errors.push(ParseError::UnexpectedEofGeneric),
            Some(_) => {
                self.builder.start_internal(Token::Error);
                self.bump();
                self.builder.finish_internal();
            }
        }
    }
    pub fn parse(mut self) -> Node {
        self.builder.start_internal(Token::Root);

        loop {
            self.newlines();

            if self.peek().is_none() {
                break;
            }

            self.parse_next();

            match self.peek() {
                None | Some(Token::Separator) => (),
                Some(_) => {
                    self.bump();
                    self.errors.push(ParseError::Trailing);
                }
            }
        }

        self.builder.finish_internal();

        Node::new(self.builder.finish(), self.errors)
    }
}

#[cfg(test)]
mod tests {
    use crate::tokenizer::Token;
    use super::*;

    use std::fmt::Write;

    fn stringify(out: &mut String, indent: usize, node: &Node) {
        writeln!(out, "{:indent$}{:?}", "", node, indent = indent).unwrap();

        for child in node.children() {
            stringify(out, indent+2, &child);
        }
    }
    fn assert(tokens: Vec<(Token, SmolStr)>, expected: &str) {
        let tokens = tokens.into_iter();
        let ast = Parser::new(tokens).parse();

        if !ast.root_data().is_empty() {
            eprintln!("root data not empty!");
            eprintln!("expected:");
            eprintln!("--------------------");
            eprintln!("{}", expected);
            eprintln!("--------------------");
            eprintln!("root data:");
            eprintln!("--------------------");
            eprintln!("{:?}", ast.root_data());
            eprintln!("--------------------");
            panic!();
        }

        let mut actual = String::new();
        stringify(&mut actual, 0, &ast);

        if actual != expected {
            eprintln!("invalid ast");
            eprintln!("expected:");
            eprintln!("--------------------");
            eprintln!("{}", expected);
            eprintln!("--------------------");
            eprintln!("found:");
            eprintln!("--------------------");
            eprintln!("{}", actual);
            eprintln!("--------------------");
            panic!();
        }
    }

    #[test]
    fn basic() {
        assert(
            vec![
                (Token::Ident, "invert".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "0".into()),
                (Token::Whitespace, " ".into()),
                (Token::Equal, "=".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "1".into()),
                (Token::Semicolon, ";".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "invert".into()),
                (Token::Whitespace, " ".into()),
                (Token::Next, "next".into())
            ],
r#"Root@[0; 25)
  Function@[0; 25)
    Type@[0; 7)
      Ident@[0; 6)
      Whitespace@[6; 7)
    Char@[7; 8)
    Whitespace@[8; 9)
    Equal@[9; 10)
    Whitespace@[10; 11)
    Char@[11; 12)
    Semicolon@[12; 13)
    Type@[13; 21)
      Whitespace@[13; 14)
      Ident@[14; 20)
      Whitespace@[20; 21)
    Move@[21; 25)
      Next@[21; 25)
"#
        );
        assert(
            vec![
                (Token::Start, "start".into()),
                (Token::Whitespace, " ".into()),
                (Token::Equal, "=".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "invert".into()),
                (Token::Separator, "\n".into()),
                (Token::Ident, "invert".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "1".into()),
                (Token::Whitespace, " ".into()),
                (Token::Equal, "=".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "0".into()),
                (Token::Semicolon, ";".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "finish".into()),
                (Token::Whitespace, " ".into()),
                (Token::Current, "current".into())
            ],
r#"Root@[0; 43)
  SetStart@[0; 14)
    Start@[0; 5)
    Whitespace@[5; 6)
    Equal@[6; 7)
    Type@[7; 14)
      Whitespace@[7; 8)
      Ident@[8; 14)
  Separator@[14; 15)
  Function@[15; 43)
    Type@[15; 22)
      Ident@[15; 21)
      Whitespace@[21; 22)
    Char@[22; 23)
    Whitespace@[23; 24)
    Equal@[24; 25)
    Whitespace@[25; 26)
    Char@[26; 27)
    Semicolon@[27; 28)
    Type@[28; 36)
      Whitespace@[28; 29)
      Ident@[29; 35)
      Whitespace@[35; 36)
    Move@[36; 43)
      Current@[36; 43)
"#
        );
    }
    #[test]
    fn generic() {
        assert(
            vec![
                (Token::Ident, "back".into()),
                (Token::AngleBOpen, "<".into()),
                (Token::Ident, "fn".into()),
                (Token::AngleBClose, ">".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "_".into()),
                (Token::Whitespace, " ".into()),
                (Token::Equal, "=".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "_".into()),
                (Token::Semicolon, ";".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "back".into()),
                (Token::AngleBOpen, "<".into()),
                (Token::Ident, "fn".into()),
                (Token::AngleBClose, ">".into()),
                (Token::Whitespace, " ".into()),
                (Token::Prev, "prev".into())
            ],
r#"Root@[0; 29)
  Function@[0; 29)
    Type@[0; 8)
      Ident@[0; 4)
      AngleBOpen@[4; 5)
      Type@[5; 7)
        Ident@[5; 7)
      AngleBClose@[7; 8)
    Whitespace@[8; 9)
    Char@[9; 10)
    Whitespace@[10; 11)
    Equal@[11; 12)
    Whitespace@[12; 13)
    Char@[13; 14)
    Semicolon@[14; 15)
    Type@[15; 24)
      Whitespace@[15; 16)
      Ident@[16; 20)
      AngleBOpen@[20; 21)
      Type@[21; 23)
        Ident@[21; 23)
      AngleBClose@[23; 24)
    Whitespace@[24; 25)
    Move@[25; 29)
      Prev@[25; 29)
"#
        );
        assert(
            vec![
                (Token::Ident, "add".into()),
                (Token::AngleBOpen, "<".into()),
                (Token::Ident, "x".into()),
                (Token::Comma, ",".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "y".into()),
                (Token::AngleBClose, ">".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "_".into()),
                (Token::Whitespace, " ".into()),
                (Token::Equal, "=".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "_".into()),
                (Token::Semicolon, ";".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "finish".into()),
                (Token::Whitespace, " ".into()),
                (Token::Current, "current".into())

            ],
r#"Root@[0; 31)
  Function@[0; 31)
    Type@[0; 9)
      Ident@[0; 3)
      AngleBOpen@[3; 4)
      Type@[4; 5)
        Ident@[4; 5)
      Comma@[5; 6)
      Whitespace@[6; 7)
      Type@[7; 8)
        Ident@[7; 8)
      AngleBClose@[8; 9)
    Whitespace@[9; 10)
    Char@[10; 11)
    Whitespace@[11; 12)
    Equal@[12; 13)
    Whitespace@[13; 14)
    Char@[14; 15)
    Semicolon@[15; 16)
    Type@[16; 24)
      Whitespace@[16; 17)
      Ident@[17; 23)
      Whitespace@[23; 24)
    Move@[24; 31)
      Current@[24; 31)
"#
        );
        assert(
            vec![
                (Token::Ident, "thing".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "_".into()),
                (Token::Whitespace, " ".into()),
                (Token::Equal, "=".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "_".into()),
                (Token::Semicolon, ";".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "add".into()),
                (Token::AngleBOpen, "<".into()),
                (Token::Ident, "num".into()),
                (Token::Comma, ",".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "sub".into()),
                (Token::AngleBOpen, "<".into()),
                (Token::Ident, "num".into()),
                (Token::Comma, ",".into()),
                (Token::Whitespace, " ".into()),
                (Token::Ident, "num".into()),
                (Token::AngleBClose, ">".into()),
                (Token::AngleBClose, ">".into()),
                (Token::Current, "current".into())
            ],
r#"Root@[0; 43)
  Function@[0; 43)
    Type@[0; 6)
      Ident@[0; 5)
      Whitespace@[5; 6)
    Char@[6; 7)
    Whitespace@[7; 8)
    Equal@[8; 9)
    Whitespace@[9; 10)
    Char@[10; 11)
    Semicolon@[11; 12)
    Type@[12; 36)
      Whitespace@[12; 13)
      Ident@[13; 16)
      AngleBOpen@[16; 17)
      Type@[17; 20)
        Ident@[17; 20)
      Comma@[20; 21)
      Whitespace@[21; 22)
      Type@[22; 35)
        Ident@[22; 25)
        AngleBOpen@[25; 26)
        Type@[26; 29)
          Ident@[26; 29)
        Comma@[29; 30)
        Whitespace@[30; 31)
        Type@[31; 34)
          Ident@[31; 34)
        AngleBClose@[34; 35)
      AngleBClose@[35; 36)
    Move@[36; 43)
      Current@[36; 43)
"#
        );
    }
}
