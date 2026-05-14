use crate::lang::{
    ast::{CondClause, Expr, Program},
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

    #[error("{op} expects {expected} argument(s), found {actual} at {span:?}")]
    WrongArity {
        op: &'static str,
        expected: usize,
        actual: usize,
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
            TokenKind::Plus => self.parse_call(Expr::AddExp),
            TokenKind::Minus => self.parse_call(Expr::SubExp),
            TokenKind::Or => self.parse_call(Expr::OrExp),
            TokenKind::Not => self.parse_not(token.span),
            TokenKind::Cond => self.parse_cond(token.span),
            found => Err(ParseError::UnexpectedToken {
                expected: "expression",
                found,
                span: token.span,
            }),
        }
    }

    fn parse_call(&mut self, build: impl FnOnce(Vec<Expr>) -> Expr) -> Result<Expr, ParseError> {
        let args = self.parse_args()?;
        Ok(build(args))
    }

    fn parse_not(&mut self, op_span: Span) -> Result<Expr, ParseError> {
        let args = self.parse_args()?;

        if args.len() != 1 {
            return Err(ParseError::WrongArity {
                op: "not",
                expected: 1,
                actual: args.len(),
                span: op_span,
            });
        }

        Ok(Expr::NotExp(Box::new(args.into_iter().next().unwrap())))
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

    #[test]
    fn parse_integer_addition() {
        assert_eq!(
            parse("+(1 2)").unwrap(),
            Program::new(Expr::AddExp(vec![Expr::ConstExp(1), Expr::ConstExp(2)]))
        );
    }

    #[test]
    fn parse_nested_integer_expression() {
        assert_eq!(
            parse("+(1 -(4 2))").unwrap(),
            Program::new(Expr::AddExp(vec![
                Expr::ConstExp(1),
                Expr::SubExp(vec![Expr::ConstExp(4), Expr::ConstExp(2)])
            ]))
        );
    }

    #[test]
    fn parse_boolean_or_and_not() {
        assert_eq!(
            parse("or(true not(false))").unwrap(),
            Program::new(Expr::OrExp(vec![
                Expr::BoolExp(true),
                Expr::NotExp(Box::new(Expr::BoolExp(false)))
            ]))
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
                    Expr::AddExp(vec![Expr::ConstExp(2), Expr::ConstExp(3)])
                )
            ]))
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
                expected: "`(`",
                found: TokenKind::Int(1),
                span: 2..3
            }
        );
    }

    #[test]
    fn parse_rejects_wrong_not_arity() {
        assert_eq!(
            parse("not(true false)").unwrap_err(),
            ParseError::WrongArity {
                op: "not",
                expected: 1,
                actual: 2,
                span: 0..3
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
}
