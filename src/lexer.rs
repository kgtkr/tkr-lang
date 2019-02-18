use crate::token::Token;

struct Lexer {
    pos: usize,
    data: Vec<char>,
    tokens: Vec<Token>,
}

impl Lexer {
    fn new(s: String) -> Lexer {
        Lexer {
            pos: 0,
            data: s.chars().collect(),
            tokens: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.data.get(self.pos).cloned()
    }

    fn peek_index(&self, i: usize) -> Option<char> {
        self.data.get(self.pos + i).cloned()
    }

    fn next(&mut self) -> Option<char> {
        let val = self.peek()?;
        self.pos += 1;
        Some(val)
    }

    fn expect(&mut self, f: impl FnOnce(char) -> bool) -> Option<char> {
        match self.peek() {
            Some(x) if f(x) => Some(x),
            _ => None,
        }
    }

    fn expect_next(&mut self, f: impl FnOnce(char) -> bool) -> Option<char> {
        let x = self.expect(f)?;
        self.next();
        Some(x)
    }

    fn spaces(&mut self) {
        while let Some(_) = self.expect(|x| x == ' ' || x == '\n' || x == '\t') {}
    }

    fn line_comment(&mut self) -> Option<()> {
        match (self.peek_index(0), self.peek_index(1)) {
            (Some('/'), Some('/')) => {
                while let Some(c) = self.peek() {
                    self.next();
                    if c == '\n' {
                        break;
                    }
                }
                Some(())
            }
            _ => None,
        }
    }

    // TODO:ネストされたブロックコメント
    fn block_comment(&mut self) -> Option<()> {
        match (self.peek_index(0), self.peek_index(1)) {
            (Some('/'), Some('*')) => {
                while let Some(c) = self.peek() {
                    self.next();
                    if c == '*' && self.peek() == Some('/') {
                        self.next();
                        break;
                    }
                }
                Some(())
            }
            _ => None,
        }
    }

    fn comment(&mut self) -> Option<()> {
        match self.line_comment() {
            None => self.block_comment(),
            x => x,
        }
    }

    fn skip(&mut self) {
        self.spaces();
        while let Some(_) = self.comment() {
            self.spaces();
        }
    }
}
