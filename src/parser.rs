use crate::ast::*;
use crate::lexer::{Lexer, Span, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn from_source(source: &str) -> Result<Self, String> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;
        Ok(Self::new(tokens))
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        let current = self.peek();
        if current.kind == kind {
            Ok(self.advance())
        } else {
            Err(format!(
                "Expected token {:?}, found {:?} at {}",
                kind, current.kind, current.span
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), String> {
        let current = self.peek().clone();
        if let TokenKind::Identifier(name) = current.kind {
            self.advance();
            Ok((name, current.span))
        } else {
            Err(format!(
                "Expected identifier, found {:?} at {}",
                current.kind, current.span
            ))
        }
    }

    pub fn parse_file(&mut self) -> Result<SourceFile, String> {
        let mut usings = Vec::new();
        let mut classes = Vec::new();

        while !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Using) {
                self.advance();
                // Support both `using "./shared/rules.brn";` and `using Shared.Rules;`
                let path = if let TokenKind::StringLiteral(ref path_str) = self.peek().kind {
                    let s = path_str.clone();
                    self.advance();
                    s
                } else {
                    let (first, _) = self.expect_ident()?;
                    let mut p = first;
                    while self.match_token(&TokenKind::Dot) {
                        let (part, _) = self.expect_ident()?;
                        p.push('.');
                        p.push_str(&part);
                    }
                    p
                };
                self.expect(TokenKind::Semicolon)?;
                usings.push(path);
            } else {
                let class_decl = self.parse_class()?;
                classes.push(class_decl);
            }
        }

        Ok(SourceFile { usings, classes })
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, String> {
        let mut attrs = Vec::new();
        while self.check(&TokenKind::OpenBracket) {
            let open = self.advance();
            let (name, _) = self.expect_ident()?;
            let mut args = Vec::new();

            if self.match_token(&TokenKind::OpenParen) {
                while !self.check(&TokenKind::CloseParen) && !self.check(&TokenKind::Eof) {
                    // Check if named argument: name = value
                    let is_named = if let TokenKind::Identifier(arg_name) = self.peek_kind().clone()
                    {
                        if self.pos + 1 < self.tokens.len()
                            && self.tokens[self.pos + 1].kind == TokenKind::Assign
                        {
                            self.advance(); // consume ident
                            self.advance(); // consume =
                            Some(arg_name)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let val = self.parse_expression()?;
                    args.push(AttributeArg {
                        name: is_named,
                        value: val,
                    });

                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::CloseParen)?;
            }

            self.expect(TokenKind::CloseBracket)?;
            attrs.push(Attribute {
                name,
                args,
                span: open.span,
            });
        }
        Ok(attrs)
    }

    fn parse_class(&mut self) -> Result<ClassDecl, String> {
        let attributes = self.parse_attributes()?;

        // Optional visibility: public
        let _is_public = self.match_token(&TokenKind::Public);

        self.expect(TokenKind::Class)?;
        let (name, span) = self.expect_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
            methods.push(self.parse_method()?);
        }

        self.expect(TokenKind::CloseBrace)?;
        Ok(ClassDecl {
            attributes,
            name,
            methods,
            span,
        })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::Void => Ok(Type::Void),
            TokenKind::String => Ok(Type::String),
            TokenKind::Bool => Ok(Type::Bool),
            TokenKind::Int => Ok(Type::Int),
            TokenKind::Identifier(name) => Ok(Type::Custom(name)),
            _ => Err(format!(
                "Expected type at {}, found {:?}",
                tok.span, tok.kind
            )),
        }
    }

    fn parse_method(&mut self) -> Result<MethodDecl, String> {
        let attributes = self.parse_attributes()?;

        let is_public = if self.match_token(&TokenKind::Public) {
            true
        } else {
            !self.match_token(&TokenKind::Private)
        };

        let return_type = self.parse_type()?;
        let (name, span) = self.expect_ident()?;

        self.expect(TokenKind::OpenParen)?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.check(&TokenKind::Eof) {
            let param_type = self.parse_type()?;
            let (param_name, p_span) = self.expect_ident()?;
            let mut default_value = None;
            if self.match_token(&TokenKind::Assign) {
                default_value = Some(self.parse_expression()?);
            }
            params.push(Parameter {
                param_type,
                name: param_name,
                default_value,
                span: p_span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::CloseParen)?;

        // Method body: either => expr; OR { stmts }
        let mut body = Vec::new();
        if self.match_token(&TokenKind::Arrow) {
            let expr = self.parse_expression()?;
            let span = expr.span();
            self.expect(TokenKind::Semicolon)?;
            body.push(Statement::Return {
                value: Some(expr),
                span,
            });
        } else {
            self.expect(TokenKind::OpenBrace)?;
            while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
                body.push(self.parse_statement()?);
            }
            self.expect(TokenKind::CloseBrace)?;
        }

        Ok(MethodDecl {
            attributes,
            is_public,
            return_type,
            name,
            params,
            body,
            span,
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        let cur = self.peek().clone();
        match cur.kind {
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Try => self.parse_try_catch(),
            TokenKind::Return => {
                let ret_tok = self.advance();
                let val = if self.check(&TokenKind::Semicolon) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Return {
                    value: val,
                    span: ret_tok.span,
                })
            }
            TokenKind::String | TokenKind::Bool | TokenKind::Int => {
                let var_type = self.parse_type()?;
                let (name, span) = self.expect_ident()?;
                let init = if self.match_token(&TokenKind::Assign) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::VarDecl {
                    var_type,
                    name,
                    init,
                    span,
                })
            }
            TokenKind::Identifier(ref id) => {
                // Check if this is a type or variable assignment or call
                // If followed by ident: Type name = expr;
                if self.pos + 1 < self.tokens.len() {
                    if let TokenKind::Identifier(_) = self.tokens[self.pos + 1].kind {
                        let var_type = self.parse_type()?;
                        let (name, span) = self.expect_ident()?;
                        let init = if self.match_token(&TokenKind::Assign) {
                            Some(self.parse_expression()?)
                        } else {
                            None
                        };
                        self.expect(TokenKind::Semicolon)?;
                        return Ok(Statement::VarDecl {
                            var_type,
                            name,
                            init,
                            span,
                        });
                    } else if self.tokens[self.pos + 1].kind == TokenKind::Assign {
                        let target = id.clone();
                        let span = cur.span;
                        self.advance(); // ident
                        self.advance(); // =
                        let value = self.parse_expression()?;
                        self.expect(TokenKind::Semicolon)?;
                        return Ok(Statement::Assignment {
                            target,
                            value,
                            span,
                        });
                    }
                }
                let expr = self.parse_expression()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Expression(expr))
            }
            _ => {
                let expr = self.parse_expression()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn parse_if(&mut self) -> Result<Statement, String> {
        let if_tok = self.advance(); // consume 'if'
        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;

        let mut then_branch = Vec::new();
        if self.match_token(&TokenKind::OpenBrace) {
            while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
                then_branch.push(self.parse_statement()?);
            }
            self.expect(TokenKind::CloseBrace)?;
        } else {
            then_branch.push(self.parse_statement()?);
        }

        let mut else_branch = None;
        if self.match_token(&TokenKind::Else) {
            let mut branch = Vec::new();
            if self.check(&TokenKind::If) {
                branch.push(self.parse_if()?);
            } else if self.match_token(&TokenKind::OpenBrace) {
                while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
                    branch.push(self.parse_statement()?);
                }
                self.expect(TokenKind::CloseBrace)?;
            } else {
                branch.push(self.parse_statement()?);
            }
            else_branch = Some(branch);
        }

        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
            span: if_tok.span,
        })
    }

    fn parse_while(&mut self) -> Result<Statement, String> {
        let while_tok = self.advance(); // consume 'while'
        self.expect(TokenKind::OpenParen)?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::CloseParen)?;

        let mut body = Vec::new();
        if self.match_token(&TokenKind::OpenBrace) {
            while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
                body.push(self.parse_statement()?);
            }
            self.expect(TokenKind::CloseBrace)?;
        } else {
            body.push(self.parse_statement()?);
        }

        Ok(Statement::While {
            condition,
            body,
            span: while_tok.span,
        })
    }

    fn parse_try_catch(&mut self) -> Result<Statement, String> {
        let try_tok = self.advance(); // consume 'try'
        self.expect(TokenKind::OpenBrace)?;
        let mut try_body = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
            try_body.push(self.parse_statement()?);
        }
        self.expect(TokenKind::CloseBrace)?;

        self.expect(TokenKind::Catch)?;
        let mut catch_exception = None;
        let mut catch_var = None;

        if self.match_token(&TokenKind::OpenParen) {
            let (exc, _) = self.expect_ident()?;
            catch_exception = Some(exc);
            if let TokenKind::Identifier(var) = self.peek_kind().clone() {
                self.advance();
                catch_var = Some(var);
            }
            self.expect(TokenKind::CloseParen)?;
        }

        self.expect(TokenKind::OpenBrace)?;
        let mut catch_body = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.check(&TokenKind::Eof) {
            catch_body.push(self.parse_statement()?);
        }
        self.expect(TokenKind::CloseBrace)?;

        Ok(Statement::TryCatch {
            try_body,
            catch_exception,
            catch_var,
            catch_body,
            span: try_tok.span,
        })
    }

    // Expressions (Pratt Parser / Precedence Climbing)
    pub fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and()?;
        while self.match_token(&TokenKind::PipePipe) {
            let span = left.span();
            let right = self.parse_and()?;
            left = Expression::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_equality()?;
        while self.match_token(&TokenKind::AmpAmp) {
            let span = left.span();
            let right = self.parse_equality()?;
            left = Expression::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_additive()?;
        loop {
            if self.match_token(&TokenKind::Equals) {
                let span = left.span();
                let right = self.parse_additive()?;
                left = Expression::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else if self.match_token(&TokenKind::NotEquals) {
                let span = left.span();
                let right = self.parse_additive()?;
                left = Expression::Binary {
                    op: BinaryOp::NotEq,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            if self.match_token(&TokenKind::Plus) {
                let span = left.span();
                let right = self.parse_multiplicative()?;
                left = Expression::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else if self.match_token(&TokenKind::Minus) {
                let span = left.span();
                let right = self.parse_multiplicative()?;
                left = Expression::Binary {
                    op: BinaryOp::Sub,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_unary()?;
        loop {
            if self.match_token(&TokenKind::Star) {
                let span = left.span();
                let right = self.parse_unary()?;
                left = Expression::Binary {
                    op: BinaryOp::Mul,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else if self.match_token(&TokenKind::Slash) {
                let span = left.span();
                let right = self.parse_unary()?;
                left = Expression::Binary {
                    op: BinaryOp::Div,
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, String> {
        if self.match_token(&TokenKind::Exclamation) {
            let span = self.tokens[self.pos - 1].span.clone();
            let expr = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            });
        }
        if self.match_token(&TokenKind::Minus) {
            let span = self.tokens[self.pos - 1].span.clone();
            let expr = self.parse_unary()?;
            return Ok(Expression::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
                span,
            });
        }
        self.parse_call_or_member()
    }

    fn parse_call_or_member(&mut self) -> Result<Expression, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&TokenKind::Dot) {
                let (member, span) = self.expect_ident()?;
                expr = Expression::MemberAccess {
                    object: Box::new(expr),
                    member,
                    span,
                };
            } else if self.match_token(&TokenKind::OpenParen) {
                let span = expr.span();
                let mut args = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expression()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::CloseParen)?;
                expr = Expression::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let cur = self.advance();
        match cur.kind {
            TokenKind::StringLiteral(s) => Ok(Expression::StringLit(s, cur.span)),
            TokenKind::InterpolatedString(raw) => self.parse_interpolated_string(&raw, cur.span),
            TokenKind::IntLiteral(i) => Ok(Expression::IntLit(i, cur.span)),
            TokenKind::True => Ok(Expression::BoolLit(true, cur.span)),
            TokenKind::False => Ok(Expression::BoolLit(false, cur.span)),
            TokenKind::Null => Ok(Expression::NullLit(cur.span)),
            TokenKind::Identifier(name) => Ok(Expression::Identifier(name, cur.span)),
            TokenKind::OpenParen => {
                let inner = self.parse_expression()?;
                self.expect(TokenKind::CloseParen)?;
                Ok(inner)
            }
            other => Err(format!("Unexpected token {:?} at {}", other, cur.span)),
        }
    }

    fn parse_interpolated_string(&self, raw: &str, span: Span) -> Result<Expression, String> {
        // Parse expressions inside {...} within $"..."
        let mut template = String::new();
        let mut exprs = Vec::new();
        let mut chars = raw.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    // Escaped {{
                    chars.next();
                    template.push('{');
                } else {
                    // Parse nested expression inside { ... }
                    let mut inner_str = String::new();
                    let mut depth = 1;
                    for ch in chars.by_ref() {
                        if ch == '{' {
                            depth += 1;
                            inner_str.push(ch);
                        } else if ch == '}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            } else {
                                inner_str.push(ch);
                            }
                        } else {
                            inner_str.push(ch);
                        }
                    }
                    if depth != 0 {
                        return Err(format!("Unclosed '{{' in interpolated string at {}", span));
                    }

                    // Parse inner expression
                    let mut sub_parser = Parser::from_source(&inner_str)?;
                    let expr = sub_parser.parse_expression()?;
                    template.push_str("{}");
                    exprs.push(expr);
                }
            } else if c == '}' && chars.peek() == Some(&'}') {
                chars.next();
                template.push('}');
            } else {
                template.push(c);
            }
        }

        Ok(Expression::InterpolatedString(template, exprs, span))
    }
}
