use std::str::Chars;

use crate::{
    strings::*,
    token::{Token, TokenKind},
};

pub struct Lexer<'a> {
    input: Chars<'a>,
    cursor: u16,
    read_cursor: u16,
    peek: char,
    current: char,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut input = input.chars();
        Self {
            cursor: 0,
            read_cursor: 1,
            current: input.next().unwrap_or(char::MIN),
            peek: input.next().unwrap_or(char::MIN),
            input: input,
        }
    }

    fn read_char(&mut self) {
        match self.current != char::MIN {
            true => {
                self.current = self.peek;
                self.peek = self.input.next().unwrap_or(char::MIN)
            }
            false => self.current = char::MIN,
        }
        self.cursor = self.read_cursor;
        self.read_cursor = self.read_cursor.saturating_add(1);
    }

    fn read_string(&mut self) {
        while self.peek.is_alphanumeric() {
            self.read_char();
        }
    }

    pub fn next_token(&mut self) -> Token {
        let token = match self.current {
            SPACE => Token::new(TokenKind::Space, self.cursor, 1),

            AT => Token::new(TokenKind::At, self.cursor, 1),

            COLON => Token::new(TokenKind::Colon, self.cursor, 1),

            SEMICOLON => Token::new(TokenKind::Semicolon, self.cursor, 1),

            EQUALS => Token::new(TokenKind::Equals, self.cursor, 1),

            BANG => Token::new(TokenKind::Bang, self.cursor, 1),

            HASH => Token::new(TokenKind::Hash, self.cursor, 1),

            SINGLE_QUOTE => Token::new(TokenKind::SingleQuote, self.cursor, 1),

            DOUBLE_QUOTE => Token::new(TokenKind::DoubleQuote, self.cursor, 1),

            SLASH => Token::new(TokenKind::Slash, self.cursor, 1),

            STAR => Token::new(TokenKind::Star, self.cursor, 1),

            CR => match self.peek.eq(&LF) {
                true => return Token::new(TokenKind::EOM, self.cursor, 2),
                false => return Token::new(TokenKind::EOM, self.cursor, 1),
            },

            LF => return Token::new(TokenKind::EOM, self.cursor, 1),

            c if c.is_alphanumeric() => {
                let start = self.cursor;
                self.read_string();
                let stop = self.read_cursor;
                Token::new(TokenKind::Text, start, stop - start)
            }

            _ => Token::new(TokenKind::Invalid, self.cursor, 0),
        };
        self.read_char();

        token
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        enable_logging,
        lexer::Lexer,
        token::{Token, TokenKind},
    };

    #[test]
    fn test_lexer1() {
        enable_logging();
        let input = "aaaa @:bbbbb ab123cd\rasdfasdf";
        let mut lexer = Lexer::new(input);

        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 0, 4));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 4, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::At, 5, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Colon, 6, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 7, 5));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 12, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 13, 7));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::EOM, 20, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::EOM, 20, 1));
    }

    #[test]
    fn test_lexer2() {
        enable_logging();
        let input = "@id=234AB :dan!d@localhost PRIVMSG #chan :Hey what's up!\r\n";
        let mut lexer = Lexer::new(input);

        assert_eq!(lexer.next_token(), Token::new(TokenKind::At, 0, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 1, 2));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Equals, 3, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 4, 5));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 9, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Colon, 10, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 11, 3));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Bang, 14, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 15, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::At, 16, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 17, 9));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 26, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 27, 7));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 34, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Hash, 35, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 36, 4));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 40, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Colon, 41, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 42, 3));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 45, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 46, 4));
        assert_eq!(
            lexer.next_token(),
            Token::new(TokenKind::SingleQuote, 50, 1)
        );
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 51, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Space, 52, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Text, 53, 2));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::Bang, 55, 1));
        assert_eq!(lexer.next_token(), Token::new(TokenKind::EOM, 56, 2));
    }
}
