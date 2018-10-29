use std::iter::Peekable;

#[derive(Clone, Copy, Debug, Fail, PartialEq, Eq)]
pub enum TokenizeError {
    //#[fail(display = "integer overflow")]
    //IntegerOverflow,
    #[fail(display = "non-ascii character as value: {:?}", _0)]
    NonAscii(char),
    #[fail(display = "unclosed character literal")]
    UnclosedChar,
    #[fail(display = "unexpected eof")]
    UnexpectedEof,
    #[fail(display = "unknown character in token")]
    UnknownCharacter
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token {
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
    Ident(String),
    Char(Option<u8>)
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: Option<u32>
}
impl Span {
    pub fn until(mut self, other: Span) -> Span {
        self.end = other.end;
        self
    }
}

type Result<T> = std::result::Result<T, (Option<Span>, TokenizeError)>;
type Item = Result<Option<(Span, Token)>>;

pub struct Tokenizer<I>
    where I: Iterator<Item = char>
{
    input: Peekable<I>,
    offset: usize
}
impl<I> Tokenizer<I>
    where I: Iterator<Item = char>
{
    pub fn new<T>(input: T) -> Self
        where T: IntoIterator<Item = I::Item, IntoIter = I>
    {
        Self {
            input: input.into_iter().peekable(),
            offset: 0
        }
    }
    fn peek(&mut self) -> Option<char> {
        self.input.peek().map(|c| *c)
    }
    fn next(&mut self) -> Result<char> {
        let next = self.input.next().ok_or((None, TokenizeError::UnexpectedEof));
        if next.is_ok() { self.offset += 1; }
        next
    }
    fn span_end(&mut self, mut span: Span, token: Token) -> Item {
        span.end = Some(self.offset as u32);
        Ok(Some((span, token)))
    }
    fn span_err(&mut self, mut span: Span, error: TokenizeError) -> Item {
        span.end = Some(self.offset as u32);
        Err((Some(span), error))
    }
    fn next_token(&mut self) -> Item {
        let mut c = self.next();
        loop {
            // Skip whitespace
            while c.map(|c| c != '\n' && c.is_whitespace()).unwrap_or(false) {
                c = self.next();
            }
            // Skip comments
            if c == Ok('/') {
                match self.peek() {
                    Some('/') => {
                        self.next().unwrap();

                        while self.next().map(|c| c != '\n').unwrap_or(false) {}
                        c = self.next();
                        continue;
                    },
                    Some('*') => {
                        self.next().unwrap();

                        loop {
                            if self.next()? == '*' {
                                if self.next()? == '/' {
                                    break;
                                }
                            }
                        }
                        c = self.next();
                        continue;
                    }
                    _ => ()
                }
            }
            break;
        }
        let c = match c {
            Ok(c) => c,
            Err(_) => return Ok(None)
        };

        // offset - 1 because we've always consumed at least one character
        let span = Span { start: (self.offset - 1) as u32, end: None };

        match c {
            '<' => self.span_end(span, Token::AngleBOpen),
            '>' => self.span_end(span, Token::AngleBClose),
            ',' => self.span_end(span, Token::Comma),
            '=' => self.span_end(span, Token::Equal),
            ';' => self.span_end(span, Token::Semicolon),
            '\n' => self.span_end(span, Token::Separator),
            '_' => self.span_end(span, Token::Char(None)),
            '\'' => {
                let c = self.next()?;

                if self.next()? != '\'' {
                    Err((Some(span), TokenizeError::UnclosedChar))
                } else if !c.is_ascii() {
                    self.span_err(span, TokenizeError::NonAscii(c))
                } else {
                    self.span_end(span, Token::Char(Some(c as u8)))
                }
            },
            '0'..='9' => self.span_end(span, Token::Char(Some(c as u8))),
            //'0'..='9' => {
            //    let mut num = c.to_digit(10).unwrap() as u8;

            //    while let Some(digit) = self.peek().and_then(|c| c.to_digit(10)) {
            //        match num.checked_mul(10).and_then(|n| n.checked_add(digit as u8)) {
            //            None => return self.span_err(span, TokenizeError::IntegerOverflow),
            //            Some(new) => num = new
            //        }
            //    }

            //    self.span_end(span, Token::Char(Some(num)))
            //},
            'a'..='z' | 'A'..='Z' => {
                let mut ident = String::new();
                ident.push(c);

                loop {
                    match self.peek() {
                        Some('a'..='z')
                        | Some('A'..='Z')
                        | Some('0'..='9')
                        | Some('_') => ident.push(self.next().unwrap()),
                        _ => break,
                    }
                }

                self.span_end(span, match &*ident {
                    "current" => Token::Current,
                    "next" => Token::Next,
                    "prev" => Token::Prev,
                    "start" => Token::Start,
                    _ => Token::Ident(ident)
                })
            },
            _ => self.span_err(span, TokenizeError::UnknownCharacter)
        }
    }
}
impl<I> Iterator for Tokenizer<I>
    where I: Iterator<Item = char>
{
    type Item = Result<(Span, Token)>;

    fn next(&mut self) -> Option<Self::Item> {
        // self.next_token().transpose()
        match self.next_token() {
            Ok(None) => None,
            Ok(Some(inner)) => Some(Ok(inner)),
            Err(err) => Some(Err(err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(s: &str) -> Vec<Token> {
        Tokenizer::new(s.chars())
            .map(|r| r.map(|(_, t)| t))
            .collect::<Result<_>>()
            .expect("failed to tokenize")
    }

    #[test]
    fn basic() {
        assert_eq!(
            tokenize("invert 0 = 1; invert next"),
            vec![
                Token::Ident("invert".into()),
                Token::Char(Some(b'0')),
                Token::Equal,
                Token::Char(Some(b'1')),
                Token::Semicolon,
                Token::Ident("invert".into()),
                Token::Next,
            ]
        );
        assert_eq!(
            tokenize("start = invert\ninvert 1 = 0; finish current"),
            vec![
                Token::Start,
                Token::Equal,
                Token::Ident("invert".into()),
                Token::Separator,
                Token::Ident("invert".into()),
                Token::Char(Some(b'1')),
                Token::Equal,
                Token::Char(Some(b'0')),
                Token::Semicolon,
                Token::Ident("finish".into()),
                Token::Current,
            ]
        );
    }
    #[test]
    fn comments() {
        assert_eq!(
            tokenize("invert /* test */ 0 // trailing..."),
            vec![
                Token::Ident("invert".into()),
                Token::Char(Some(b'0'))
            ]
        );
    }
    #[test]
    fn generic() {
        assert_eq!(
            tokenize("back<fn> _ = _; back<fn> prev"),
            vec![
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
            ]
        );
        assert_eq!(
            tokenize("add<x, y>"),
            vec![
                Token::Ident("add".into()),
                Token::AngleBOpen,
                Token::Ident("x".into()),
                Token::Comma,
                Token::Ident("y".into()),
                Token::AngleBClose
            ]
        );
    }
    #[test]
    fn spans() {
        assert_eq!(
            Tokenizer::new("invert 0 = /* testing */ 1; invert next".chars()).collect::<Result<_>>(),
            Ok(vec![
               (Span { start: 0, end: Some(6) }, Token::Ident("invert".into())),
               (Span { start: 7, end: Some(8) }, Token::Char(Some(b'0'))),
               (Span { start: 9, end: Some(10) }, Token::Equal),
               (Span { start: 25, end: Some(26) }, Token::Char(Some(b'1'))),
               (Span { start: 26, end: Some(27) }, Token::Semicolon),
               (Span { start: 28, end: Some(34) }, Token::Ident("invert".into())),
               (Span { start: 35, end: Some(39) }, Token::Next),
            ])
        );
        assert_eq!(
            Tokenizer::new("\'ä\'".chars()).collect::<Result<Vec<(Span, Token)>>>(),
            Err((Some(Span { start: 0, end: Some(3) }), TokenizeError::NonAscii('ä')))
        );
    }
}
