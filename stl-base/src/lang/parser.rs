use crate::lang::{
    ast::{CondClause, Expr, LetBinding, LetRecBinding, Program},
    scanner::{ScanError, Span, Token, TokenKind, scan},
};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    Scan(#[from] ScanError),

    #[error("expected {expected}, found end of input")]
    UnexpectedEnd { expected: &'static str },

    #[error("expected {expected}, found {found:?} at {span:?}")]
    UnexpectedToken {
        expected: &'static str,
        found: TokenKind,
        span: Span,
    },

    #[error("cond expects at least one clause at {span:?}")]
    EmptyCond { span: Span },
}

pub fn parse(input: &str) -> Result<Program, ParseError> {
    let tokens = scan(input)?;
    Parser::new(tokens).parse_program()
}

#[derive(Clone, Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse_program(mut self) -> Result<Program, ParseError> {
        let body = self.parse_expr()?;

        if let Some(token) = self.peek() {
            return Err(ParseError::UnexpectedToken {
                expected: "end of input",
                found: token.kind.clone(),
                span: token.span.clone(),
            });
        }

        Ok(Program::new(body))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance("expression")?;

        match token.kind {
            TokenKind::Int(n) => Ok(Expr::ConstExp(n)),
            TokenKind::True => Ok(Expr::BoolExp(true)),
            TokenKind::False => Ok(Expr::BoolExp(false)),
            TokenKind::Ident(var) => self.parse_var_or_call(var),
            TokenKind::Plus => self.parse_var_or_call("+"),
            TokenKind::Minus => self.parse_var_or_call("-"),
            TokenKind::Equals => self.parse_var_or_call("="),
            TokenKind::Not => self.parse_var_or_call("not"),
            TokenKind::Or => self.parse_call(Expr::OrExp),
            TokenKind::Cond => self.parse_cond(token.span),
            TokenKind::Let => self.parse_let(),
            TokenKind::LetRec => self.parse_letrec(),
            TokenKind::Fn => self.parse_fn(),
            found => Err(ParseError::UnexpectedToken {
                expected: "expression",
                found,
                span: token.span,
            }),
        }
    }

    fn parse_var_or_call(&mut self, var: impl Into<String>) -> Result<Expr, ParseError> {
        let operator = Expr::VarExp(var.into());

        if self.check_lparen() {
            let operands = self.parse_args()?;

            Ok(Expr::CallExp {
                operator: Box::new(operator),
                operands,
            })
        } else {
            Ok(operator)
        }
    }

    fn parse_call(&mut self, build: impl FnOnce(Vec<Expr>) -> Expr) -> Result<Expr, ParseError> {
        let args = self.parse_args()?;
        Ok(build(args))
    }

    fn parse_cond(&mut self, cond_span: Span) -> Result<Expr, ParseError> {
        self.expect_lparen()?;

        let mut clauses = Vec::new();

        while !self.check_rparen() {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEnd { expected: "`)`" });
            }

            let condition = self.parse_expr()?;
            self.expect_arrow()?;
            let result = self.parse_expr()?;
            clauses.push(CondClause::new(condition, result));
        }

        self.expect_rparen()?;

        if clauses.is_empty() {
            return Err(ParseError::EmptyCond { span: cond_span });
        }

        Ok(Expr::CondExp(clauses))
    }

    fn parse_let(&mut self) -> Result<Expr, ParseError> {
        self.expect_lparen()?;

        let mut bindings = Vec::new();

        while !self.check_rparen() {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEnd { expected: "`)`" });
            }

            let var = self.expect_ident()?;
            self.expect_equals()?;
            let expr = self.parse_expr()?;
            bindings.push(LetBinding::new(var, expr));
        }

        self.expect_rparen()?;
        let body = self.parse_expr()?;

        Ok(Expr::LetExp {
            bindings,
            body: Box::new(body),
        })
    }

    fn parse_fn(&mut self) -> Result<Expr, ParseError> {
        let (params, body) = self.parse_fn_parts()?;

        Ok(Expr::FnExp {
            params,
            body: Box::new(body),
        })
    }

    fn parse_letrec(&mut self) -> Result<Expr, ParseError> {
        self.expect_lparen()?;

        let mut bindings = Vec::new();

        while !self.check_rparen() {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEnd { expected: "`)`" });
            }

            let name = self.expect_ident()?;
            self.expect_equals()?;
            self.expect_fn()?;
            let (params, body) = self.parse_fn_parts()?;
            bindings.push(LetRecBinding::new(name, params, body));
        }

        self.expect_rparen()?;
        let body = self.parse_expr()?;

        Ok(Expr::LetRecExp {
            bindings,
            body: Box::new(body),
        })
    }

    fn parse_fn_parts(&mut self) -> Result<(Vec<String>, Expr), ParseError> {
        self.expect_lparen()?;

        let mut params = Vec::new();

        while !self.check_rparen() {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEnd { expected: "`)`" });
            }

            params.push(self.expect_ident()?);
        }

        self.expect_rparen()?;
        let body = self.parse_expr()?;

        Ok((params, body))
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        self.expect_lparen()?;

        let mut args = Vec::new();

        while !self.check_rparen() {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEnd { expected: "`)`" });
            }

            args.push(self.parse_expr()?);
        }

        self.expect_rparen()?;
        Ok(args)
    }

    fn expect_lparen(&mut self) -> Result<(), ParseError> {
        let token = self.advance("`(`")?;

        match token.kind {
            TokenKind::LParen => Ok(()),
            found => Err(ParseError::UnexpectedToken {
                expected: "`(`",
                found,
                span: token.span,
            }),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), ParseError> {
        let token = self.advance("`)`")?;

        match token.kind {
            TokenKind::RParen => Ok(()),
            found => Err(ParseError::UnexpectedToken {
                expected: "`)`",
                found,
                span: token.span,
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        let token = self.advance("identifier")?;

        match token.kind {
            TokenKind::Ident(var) => Ok(var),
            found => Err(ParseError::UnexpectedToken {
                expected: "identifier",
                found,
                span: token.span,
            }),
        }
    }

    fn expect_equals(&mut self) -> Result<(), ParseError> {
        let token = self.advance("`=`")?;

        match token.kind {
            TokenKind::Equals => Ok(()),
            found => Err(ParseError::UnexpectedToken {
                expected: "`=`",
                found,
                span: token.span,
            }),
        }
    }

    fn expect_fn(&mut self) -> Result<(), ParseError> {
        let token = self.advance("`fn`")?;

        match token.kind {
            TokenKind::Fn => Ok(()),
            found => Err(ParseError::UnexpectedToken {
                expected: "`fn`",
                found,
                span: token.span,
            }),
        }
    }

    fn expect_arrow(&mut self) -> Result<(), ParseError> {
        let token = self.advance("`=>`")?;

        match token.kind {
            TokenKind::Arrow => Ok(()),
            found => Err(ParseError::UnexpectedToken {
                expected: "`=>`",
                found,
                span: token.span,
            }),
        }
    }

    fn check_rparen(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::RParen,
                ..
            })
        )
    }

    fn check_lparen(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::LParen,
                ..
            })
        )
    }

    fn advance(&mut self, expected: &'static str) -> Result<Token, ParseError> {
        let token = self
            .tokens
            .get(self.current)
            .cloned()
            .ok_or(ParseError::UnexpectedEnd { expected })?;

        self.current += 1;
        Ok(token)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn call(operator: Expr, operands: Vec<Expr>) -> Expr {
        Expr::CallExp {
            operator: Box::new(operator),
            operands,
        }
    }

    fn var(name: &str) -> Expr {
        Expr::VarExp(name.to_string())
    }

    #[test]
    fn parse_integer_addition() {
        assert_eq!(
            parse("+(1 2)").unwrap(),
            Program::new(call(var("+"), vec![Expr::ConstExp(1), Expr::ConstExp(2)]))
        );
    }

    #[test]
    fn parse_nested_integer_expression() {
        assert_eq!(
            parse("+(1 -(4 2))").unwrap(),
            Program::new(call(
                var("+"),
                vec![
                    Expr::ConstExp(1),
                    call(var("-"), vec![Expr::ConstExp(4), Expr::ConstExp(2)])
                ]
            ))
        );
    }

    #[test]
    fn parse_boolean_or_and_not() {
        assert_eq!(
            parse("or(true not(false))").unwrap(),
            Program::new(Expr::OrExp(vec![
                Expr::BoolExp(true),
                call(var("not"), vec![Expr::BoolExp(false)])
            ]))
        );
    }

    #[test]
    fn parse_equality_call() {
        assert_eq!(
            parse("=(1 2)").unwrap(),
            Program::new(call(var("="), vec![Expr::ConstExp(1), Expr::ConstExp(2)]))
        );
    }

    #[test]
    fn parse_cond_expression() {
        assert_eq!(
            parse(
                "cond (
                    false => 1
                    true => +(2 3)
                )"
            )
            .unwrap(),
            Program::new(Expr::CondExp(vec![
                CondClause::new(Expr::BoolExp(false), Expr::ConstExp(1)),
                CondClause::new(
                    Expr::BoolExp(true),
                    call(var("+"), vec![Expr::ConstExp(2), Expr::ConstExp(3)])
                )
            ]))
        );
    }

    #[test]
    fn parse_variable_reference() {
        assert_eq!(parse("x").unwrap(), Program::new(var("x")));
    }

    #[test]
    fn parse_function_expression() {
        assert_eq!(
            parse("fn(x y) +(x y)").unwrap(),
            Program::new(Expr::FnExp {
                params: vec!["x".to_string(), "y".to_string()],
                body: Box::new(call(var("+"), vec![var("x"), var("y")]))
            })
        );
    }

    #[test]
    fn parse_function_call() {
        assert_eq!(
            parse("f(1 2)").unwrap(),
            Program::new(call(var("f"), vec![Expr::ConstExp(1), Expr::ConstExp(2)]))
        );
    }

    #[test]
    fn parse_let_expression() {
        assert_eq!(
            parse(
                "let (
                    x = 1
                    y = +(2 3)
                ) +(x y)"
            )
            .unwrap(),
            Program::new(Expr::LetExp {
                bindings: vec![
                    LetBinding::new("x", Expr::ConstExp(1)),
                    LetBinding::new(
                        "y",
                        call(var("+"), vec![Expr::ConstExp(2), Expr::ConstExp(3)])
                    )
                ],
                body: Box::new(call(var("+"), vec![var("x"), var("y")]))
            })
        );
    }

    #[test]
    fn parse_letrec_expression() {
        assert_eq!(
            parse(
                "letrec (
                    f = fn(x) +(x 1)
                    g = fn(y) f(y)
                ) g(41)"
            )
            .unwrap(),
            Program::new(Expr::LetRecExp {
                bindings: vec![
                    LetRecBinding::new(
                        "f",
                        vec!["x".to_string()],
                        call(var("+"), vec![var("x"), Expr::ConstExp(1)])
                    ),
                    LetRecBinding::new("g", vec!["y".to_string()], call(var("f"), vec![var("y")]))
                ],
                body: Box::new(call(var("g"), vec![Expr::ConstExp(41)]))
            })
        );
    }

    #[test]
    fn parse_rejects_extra_input() {
        assert_eq!(
            parse("true false").unwrap_err(),
            ParseError::UnexpectedToken {
                expected: "end of input",
                found: TokenKind::False,
                span: 5..10
            }
        );
    }

    #[test]
    fn parse_requires_call_parentheses() {
        assert_eq!(
            parse("+ 1 2").unwrap_err(),
            ParseError::UnexpectedToken {
                expected: "end of input",
                found: TokenKind::Int(1),
                span: 2..3
            }
        );
    }

    #[test]
    fn parse_rejects_cond_clause_without_arrow() {
        assert_eq!(
            parse("cond (true 1)").unwrap_err(),
            ParseError::UnexpectedToken {
                expected: "`=>`",
                found: TokenKind::Int(1),
                span: 11..12
            }
        );
    }

    #[test]
    fn parse_rejects_empty_cond() {
        assert_eq!(
            parse("cond ()").unwrap_err(),
            ParseError::EmptyCond { span: 0..4 }
        );
    }

    #[test]
    fn parse_rejects_let_binding_without_equals() {
        assert_eq!(
            parse("let (x 1) x").unwrap_err(),
            ParseError::UnexpectedToken {
                expected: "`=`",
                found: TokenKind::Int(1),
                span: 7..8
            }
        );
    }

    #[test]
    fn parse_rejects_letrec_binding_without_fn() {
        assert_eq!(
            parse("letrec (x = 1) x").unwrap_err(),
            ParseError::UnexpectedToken {
                expected: "`fn`",
                found: TokenKind::Int(1),
                span: 12..13
            }
        );
    }
}
