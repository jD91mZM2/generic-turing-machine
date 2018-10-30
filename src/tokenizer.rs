use rowan::SmolStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    // Meta
    Comment,
    Error,
    Whitespace,

    // Internal
    Function,
    Move,
    Root,
    SetStart,
    Type,

    // Characters
    AngleBClose,
    AngleBOpen,
    Comma,
    Equal,
    Semicolon,
    Separator,

    // Keywords
    Current,
    Next,
    Prev,
    Start,

    // Values
    Ident,
    Char
}
impl Token {
    pub fn is_trivia(self) -> bool {
        match self {
            Token::Comment | Token::Error | Token::Whitespace => true,
            _ => false
        }
    }
    pub fn is_move(self) -> bool {
        match self {
            Token::Current | Token::Next | Token::Prev => true,
            _ => false
        }
    }
}

#[derive(Clone, Copy)]
pub struct Tokenizer<'a> {
    input: &'a str,
    offset: usize
}
impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0
        }
    }
    fn peek(&self) -> Option<char> {
        self.input[self.offset..].chars().next()
    }
    fn next(&mut self) -> Option<char> {
        let next = self.input[self.offset..].chars().next()?;
        self.offset += next.len_utf8();
        Some(next)
    }
}
impl<'a> Iterator for Tokenizer<'a> {
    type Item = (Token, SmolStr);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = *self;
        while self.peek().map(|c| c.is_whitespace() && c != '\n').unwrap_or(false) {
            self.next().unwrap();
        }
        if self.offset > start.offset {
            let s = SmolStr::new(&start.input[start.offset..self.offset]);
            return Some((Token::Whitespace, s))
        }
        start = *self;
        let c = self.next()?;

        match c {
            '<' => Some((Token::AngleBOpen, SmolStr::new("<"))),
            '>' => Some((Token::AngleBClose, SmolStr::new(">"))),
            ',' => Some((Token::Comma, SmolStr::new(","))),
            '=' => Some((Token::Equal, SmolStr::new("="))),
            ';' => Some((Token::Semicolon, SmolStr::new(";"))),
            '\n' => Some((Token::Separator, SmolStr::new("\n"))),
            '_' => Some((Token::Char, SmolStr::new("_"))),
            '/' if self.peek() == Some('*') => {
                self.next().unwrap();

                let mut ended = false;
                while self.peek().is_some() {
                    if self.next() == Some('*') {
                        if self.next() == Some('/') {
                            ended = true;
                            break;
                        }
                    }
                }
                Some((
                    if ended { Token::Comment } else { Token::Error },
                    SmolStr::new(&start.input[start.offset..self.offset])
                ))
            },
            '/' if self.peek() == Some('/') => {
                self.next().unwrap();

                while self.next().map(|c| c != '\n').unwrap_or(false) {}
                Some((Token::Comment, SmolStr::new(&start.input[start.offset..self.offset])))
            },
            '\'' => {
                let c = self.next()?;

                if self.next()? != '\'' {
                    Some((Token::Error, SmolStr::new(&start.input[start.offset..self.offset])))
                } else if !c.is_ascii() {
                    Some((Token::Error, SmolStr::new(&start.input[start.offset..self.offset])))
                } else {
                    Some((Token::Char, SmolStr::new(&start.input[start.offset..self.offset])))
                }
            },
            '0'..='9' => Some((Token::Char, SmolStr::new(&start.input[start.offset..self.offset]))),
            'a'..='z' | 'A'..='Z' => {
                loop {
                    match self.peek() {
                        Some('a'..='z')
                        | Some('A'..='Z')
                        | Some('0'..='9')
                        | Some('_') => {
                            self.next().unwrap();
                        },
                        _ => break,
                    }
                }

                let s = &start.input[start.offset..self.offset];

                Some((match s {
                    "current" => Token::Current,
                    "next" => Token::Next,
                    "prev" => Token::Prev,
                    "start" => Token::Start,
                    _ => Token::Ident
                }, SmolStr::new(s)))
            },
            _ => Some((Token::Error, SmolStr::new(&start.input[start.offset..self.offset])))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(s: &str) -> Vec<(Token, SmolStr)> {
        Tokenizer::new(s).collect()
    }

    #[test]
    fn basic() {
        assert_eq!(
            tokenize("invert 0 = 1; invert next"),
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
            ]
        );
        assert_eq!(
            tokenize("start = invert\ninvert 1 = 0; finish current"),
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
            ]
        );
    }
    #[test]
    fn comments() {
        assert_eq!(
            tokenize("// hi\ninvert /* test */ 0 // trailing..."),
            vec![
                (Token::Comment, "// hi\n".into()),
                (Token::Ident, "invert".into()),
                (Token::Whitespace, " ".into()),
                (Token::Comment, "/* test */".into()),
                (Token::Whitespace, " ".into()),
                (Token::Char, "0".into()),
                (Token::Whitespace, " ".into()),
                (Token::Comment, "// trailing...".into())
            ]
        );
    }
    #[test]
    fn generic() {
        assert_eq!(
            tokenize("back<fn> _ = _; back<fn> prev"),
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
            ]
        );
        assert_eq!(
            tokenize("add<x, y> _ = _; finish current"),
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
            ]
        );
        assert_eq!(
            tokenize("thing _ = _; add<num, sub<num, num>> current"),
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
                (Token::Whitespace, " ".into()),
                (Token::Current, "current".into())
            ]
        );
    }
    #[test]
    fn errors() {
        assert_eq!(
            tokenize("'ä'"),
            vec![
                (Token::Error, "'ä'".into())
            ]
        );
    }
}
