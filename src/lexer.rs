use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Using,
    Public,
    Private,
    Class,
    Void,
    String,
    Bool,
    Int,
    If,
    Else,
    While,
    Try,
    Catch,
    Return,
    True,
    False,
    Null,

    // Identifiers & Literals
    Identifier(String),
    StringLiteral(String),
    InterpolatedString(String), // Raw template string content inside $"..."
    IntLiteral(i64),

    // Symbols & Operators
    OpenBrace,    // {
    CloseBrace,   // }
    OpenParen,    // (
    CloseParen,   // )
    OpenBracket,  // [
    CloseBracket, // ]
    Semicolon,    // ;
    Comma,        // ,
    Dot,          // .
    Colon,        // :
    Assign,       // =
    Equals,       // ==
    NotEquals,    // !=
    Exclamation,  // !
    AmpAmp,       // &&
    PipePipe,     // ||
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Arrow,        // =>

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    _input: &'a str,
    chars: Vec<(usize, char)>,
    index: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            _input: input,
            chars: input.char_indices().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).map(|&(_, c)| c)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.index + 1).map(|&(_, c)| c)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(&(_, c)) = self.chars.get(self.index) {
            self.index += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' && self.peek_next() == Some('/') {
                // Single-line comment
                self.advance();
                self.advance();
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        self.advance();
                        break;
                    }
                    self.advance();
                }
            } else if c == '/' && self.peek_next() == Some('*') {
                // Multi-line comment
                self.advance();
                self.advance();
                while let Some(ch) = self.peek() {
                    if ch == '*' && self.peek_next() == Some('/') {
                        self.advance();
                        self.advance();
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace_and_comments();

        let span = Span::new(self.line, self.column);
        let Some(c) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span,
            });
        };

        // Interpolated string $"..."
        if c == '$' && self.peek_next() == Some('"') {
            self.advance(); // consume $
            self.advance(); // consume "
            let mut content = String::new();
            while let Some(ch) = self.peek() {
                if ch == '"' {
                    self.advance();
                    return Ok(Token {
                        kind: TokenKind::InterpolatedString(content),
                        span,
                    });
                } else if ch == '\\' {
                    self.advance();
                    if let Some(escaped) = self.advance() {
                        content.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            other => other,
                        });
                    }
                } else {
                    content.push(ch);
                    self.advance();
                }
            }
            return Err(format!("Unterminated interpolated string at {}", span));
        }

        // Regular String literal "..."
        if c == '"' {
            self.advance(); // consume "
            let mut content = String::new();
            while let Some(ch) = self.peek() {
                if ch == '"' {
                    self.advance();
                    return Ok(Token {
                        kind: TokenKind::StringLiteral(content),
                        span,
                    });
                } else if ch == '\\' {
                    self.advance();
                    if let Some(escaped) = self.advance() {
                        content.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            other => other,
                        });
                    }
                } else {
                    content.push(ch);
                    self.advance();
                }
            }
            return Err(format!("Unterminated string literal at {}", span));
        }

        // Numbers
        if c.is_ascii_digit() {
            let mut num_str = String::new();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    num_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            let val = num_str
                .parse::<i64>()
                .map_err(|e| format!("Invalid integer at {}: {}", span, e))?;
            return Ok(Token {
                kind: TokenKind::IntLiteral(val),
                span,
            });
        }

        // Identifiers & Keywords
        if c.is_alphabetic() || c == '_' {
            let mut ident = String::new();
            while let Some(ch) = self.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    ident.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }

            let kind = match ident.as_str() {
                "using" => TokenKind::Using,
                "public" => TokenKind::Public,
                "private" => TokenKind::Private,
                "class" => TokenKind::Class,
                "void" => TokenKind::Void,
                "string" => TokenKind::String,
                "bool" => TokenKind::Bool,
                "int" => TokenKind::Int,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "while" => TokenKind::While,
                "try" => TokenKind::Try,
                "catch" => TokenKind::Catch,
                "return" => TokenKind::Return,
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                "null" => TokenKind::Null,
                _ => TokenKind::Identifier(ident),
            };

            return Ok(Token { kind, span });
        }

        // Symbols and Operators
        self.advance();
        let kind = match c {
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            ':' => TokenKind::Colon,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Equals
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::NotEquals
                } else {
                    TokenKind::Exclamation
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    TokenKind::AmpAmp
                } else {
                    return Err(format!("Unexpected character '&' at {}", span));
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    TokenKind::PipePipe
                } else {
                    return Err(format!("Unexpected character '|' at {}", span));
                }
            }
            other => return Err(format!("Unexpected character '{}' at {}", other, span)),
        };

        Ok(Token { kind, span })
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}
