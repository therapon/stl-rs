use crate::lang::{
    eval::{EvalError, Evaluator},
    parser::{ParseError, parse},
    vals::ExpVal,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expected {
    Value(ExpVal),
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Actual {
    Value(ExpVal),
    ParseError(ParseError),
    EvalError(EvalError),
}

pub fn value(value: ExpVal) -> Expected {
    Expected::Value(value)
}

pub fn error() -> Expected {
    Expected::Error
}

pub fn check(source: &str, expected: Expected) {
    let actual = parse(source)
        .map(|program| Evaluator::new().eval_program(&program))
        .map_or_else(Actual::ParseError, |result| match result {
            Ok(value) => Actual::Value(value),
            Err(error) => Actual::EvalError(error),
        });

    match (&actual, &expected) {
        (Actual::Value(actual), Expected::Value(expected)) if actual == expected => {}
        (Actual::ParseError(_) | Actual::EvalError(_), Expected::Error) => {}
        _ => panic!(
            "language check failed\n\nsource:\n{}\n\nexpected:\n{}\n\nactual:\n{}",
            source.trim(),
            format_expected(&expected),
            format_actual(&actual),
        ),
    }
}

fn format_expected(expected: &Expected) -> String {
    match expected {
        Expected::Value(value) => format!("value {value}"),
        Expected::Error => "error".to_string(),
    }
}

fn format_actual(actual: &Actual) -> String {
    match actual {
        Actual::Value(value) => format!("value {value}"),
        Actual::ParseError(error) => format!("parse error: {error}"),
        Actual::EvalError(error) => format!("eval error: {error}"),
    }
}
