use std::slice::Iter;

use crate::{
    strings,
    token::{Token, TokenKind},
};

pub(crate) struct Lexer<'a> {
    input: Iter<'a, u8>,
    cursor: u16,
    read_cursor: u16,
    peek: &'a u8,
    current: &'a u8,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        let mut input = input.as_bytes().iter();
        Self {
            cursor: 0,
            read_cursor: 1,
            current: input.next().unwrap_or(&u8::MIN),
            peek: input.next().unwrap_or(&u8::MIN),
            input,
        }
    }

    fn read_char(&mut self) {
        if *self.current != u8::MIN {
            self.current = self.peek;
            self.peek = self.input.next().unwrap_or(&u8::MIN)
        }
        self.cursor = self.read_cursor;
        self.read_cursor = self.read_cursor.saturating_add(1);
    }

    fn read_string(&mut self) {
        while self.peek.is_ascii_alphanumeric() {
            self.read_char();
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = match *self.current {
            strings::SPACE => Token::new(TokenKind::Space, self.cursor, 1),
            strings::AT => Token::new(TokenKind::At, self.cursor, 1),
            strings::COLON => Token::new(TokenKind::Colon, self.cursor, 1),
            strings::SEMICOLON => Token::new(TokenKind::Semicolon, self.cursor, 1),
            strings::LESSER => Token::new(TokenKind::Lesser, self.cursor, 1),
            strings::EQUALS => Token::new(TokenKind::Equals, self.cursor, 1),
            strings::GREATER => Token::new(TokenKind::Greater, self.cursor, 1),
            strings::BANG => Token::new(TokenKind::Bang, self.cursor, 1),
            strings::QUESTION_MARK => Token::new(TokenKind::QuestionMark, self.cursor, 1),
            strings::HASH => Token::new(TokenKind::Hash, self.cursor, 1),
            strings::SINGLE_QUOTE => Token::new(TokenKind::SingleQuote, self.cursor, 1),
            strings::DOUBLE_QUOTE => Token::new(TokenKind::DoubleQuote, self.cursor, 1),
            strings::SLASH => Token::new(TokenKind::Slash, self.cursor, 1),
            strings::ASTERISK => Token::new(TokenKind::Asterisk, self.cursor, 1),
            strings::PERIOD => Token::new(TokenKind::Period, self.cursor, 1),
            strings::COMMA => Token::new(TokenKind::Comma, self.cursor, 1),
            strings::DASH => Token::new(TokenKind::Dash, self.cursor, 1),
            strings::MINUS => Token::new(TokenKind::Minus, self.cursor, 1),
            strings::PLUS => Token::new(TokenKind::Plus, self.cursor, 1),
            strings::DOLLAR_SIGN => Token::new(TokenKind::DollarSign, self.cursor, 1),
            strings::CR => {
                self.current = &strings::NULL;
                match self.peek.eq(&strings::LF) {
                    true => return Some(Token::new(TokenKind::EndOfMessage, self.cursor, 2)),
                    false => return Some(Token::new(TokenKind::EndOfMessage, self.cursor, 1)),
                }
            }
            strings::LF => {
                self.current = &strings::NULL;
                return Some(Token::new(TokenKind::EndOfMessage, self.cursor, 1));
            }

            strings::NULL => return None,

            c if c.is_ascii_alphanumeric() => {
                let start = self.cursor;
                self.read_string();
                let stop = self.read_cursor;
                Token::new(TokenKind::Text, start, stop - start)
            }

            _ => {
                return None;
            }
        };
        self.read_char();

        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        lexer::Lexer,
        token::{Token, TokenKind},
    };

    #[test]
    fn test_lexer1() {
        let input = "aaaa @:bbbbb ab123cd\rasdfasdf";
        let mut lexer = Lexer::new(input);

        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 0, 4));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 4, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::At, 5, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Colon, 6, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 7, 5));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 12, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 13, 7));
        assert_eq!(
            lexer.next().unwrap(),
            Token::new(TokenKind::EndOfMessage, 20, 1)
        );
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_lexer2() {
        let input = "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n";
        let mut lexer = Lexer::new(input);

        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::At, 0, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 1, 2));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Equals, 3, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 4, 5));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 9, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Colon, 10, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 11, 3));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Bang, 14, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 15, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::At, 16, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 17, 9));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 26, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 27, 7));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 34, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Hash, 35, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 36, 4));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 40, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Colon, 41, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 42, 3));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 45, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 46, 4));
        assert_eq!(
            lexer.next().unwrap(),
            Token::new(TokenKind::SingleQuote, 50, 1)
        );
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 51, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Space, 52, 1));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 53, 2));
        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Bang, 55, 1));
        assert_eq!(
            lexer.next().unwrap(),
            Token::new(TokenKind::EndOfMessage, 56, 2)
        );
    }

    #[test]
    fn test_lexer3() {
        let input = "a^b";
        let mut lexer = Lexer::new(input);

        assert_eq!(lexer.next().unwrap(), Token::new(TokenKind::Text, 0, 1));
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_lexer4() {
        let input = "";
        let mut lexer = Lexer::new(input);

        assert_eq!(lexer.next(), None);
    }
}
