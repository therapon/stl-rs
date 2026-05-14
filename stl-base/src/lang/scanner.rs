use std::ops::Range;

use logos::Logos;

pub type Span = Range<usize>;

#[derive(Logos, Clone, Debug, PartialEq, Eq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum TokenKind {
    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("=>")]
    Arrow,

    #[token("cond")]
    Cond,

    #[token("or")]
    Or,

    #[token("not")]
    Not,

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().expect("integer regex should parse"))]
    Int(i64),

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ScanError {
    #[error("unexpected token at {span:?}")]
    UnexpectedToken { span: Span },
}

pub fn scan(input: &str) -> Result<Vec<Token>, ScanError> {
    TokenKind::lexer(input)
        .spanned()
        .map(|(result, span)| {
            let kind = result.map_err(|_| ScanError::UnexpectedToken { span: span.clone() })?;

            Ok(Token { kind, span })
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn scan_tracks_spans_through_skipped_whitespace() {
        let tokens = scan(" +(1\n 2)").unwrap();

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::Plus,
                    span: 1..2
                },
                Token {
                    kind: TokenKind::LParen,
                    span: 2..3
                },
                Token {
                    kind: TokenKind::Int(1),
                    span: 3..4
                },
                Token {
                    kind: TokenKind::Int(2),
                    span: 6..7
                },
                Token {
                    kind: TokenKind::RParen,
                    span: 7..8
                },
            ]
        );
    }

    #[test]
    fn scan_rejects_unknown_punctuation() {
        assert_eq!(
            scan("+(1, 2)").unwrap_err(),
            ScanError::UnexpectedToken { span: 3..4 }
        );
    }

    #[test]
    fn scan_keeps_unknown_words_as_identifiers() {
        let tokens = scan("orange").unwrap();

        assert_eq!(
            tokens,
            vec![Token {
                kind: TokenKind::Ident("orange".to_string()),
                span: 0..6
            }]
        );
    }

    #[test]
    fn scan_cond_tokens() {
        let tokens = scan("cond (true => 1)").unwrap();

        assert_eq!(
            tokens,
            vec![
                Token {
                    kind: TokenKind::Cond,
                    span: 0..4
                },
                Token {
                    kind: TokenKind::LParen,
                    span: 5..6
                },
                Token {
                    kind: TokenKind::True,
                    span: 6..10
                },
                Token {
                    kind: TokenKind::Arrow,
                    span: 11..13
                },
                Token {
                    kind: TokenKind::Int(1),
                    span: 14..15
                },
                Token {
                    kind: TokenKind::RParen,
                    span: 15..16
                },
            ]
        );
    }
}
