use miette::{IntoDiagnostic, LabeledSpan, MietteDiagnostic, NamedSource, Report, Result};
use reedline::{
    DefaultPrompt, DefaultPromptSegment, Reedline, Signal, ValidationResult, Validator,
};

use crate::lang::{
    eval::{EvalError, Evaluator},
    parser::{ParseError, parse},
    scanner::ScanError,
};

pub fn run() -> Result<()> {
    let mut line_editor = Reedline::create().with_validator(Box::new(LanguageValidator));
    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic("stl".to_string()),
        DefaultPromptSegment::Empty,
    );
    let evaluator = Evaluator::new();

    loop {
        match line_editor.read_line(&prompt).into_diagnostic()? {
            Signal::Success(input) => eval_and_print(&evaluator, &input),
            Signal::CtrlD | Signal::CtrlC => break,
            signal => eprintln!("{signal:?}"),
        }
    }

    Ok(())
}

fn eval_and_print(evaluator: &Evaluator, input: &str) {
    if input.trim().is_empty() {
        return;
    }

    match parse(input) {
        Ok(program) => match evaluator.eval_program(&program) {
            Ok(val) => println!("{val}"),
            Err(error) => print_report(eval_error_report(error)),
        },
        Err(error) => print_report(parse_error_report(input, error)),
    }
}

fn print_report(report: Report) {
    eprintln!("{report:?}");
}

fn parse_error_report(input: &str, error: ParseError) -> Report {
    let diagnostic = match error {
        ParseError::Scan(ScanError::UnexpectedToken { span }) => {
            MietteDiagnostic::new("unexpected token")
                .with_code("stl::scan")
                .with_label(LabeledSpan::at(span, "unexpected token"))
        }
        ParseError::UnexpectedEnd { expected } => {
            MietteDiagnostic::new(format!("expected {expected}, found end of input"))
                .with_code("stl::parse")
                .with_label(LabeledSpan::at_offset(
                    input.len(),
                    format!("expected {expected}"),
                ))
        }
        ParseError::UnexpectedToken {
            expected,
            found,
            span,
        } => MietteDiagnostic::new(format!("expected {expected}, found {found:?}"))
            .with_code("stl::parse")
            .with_label(LabeledSpan::at(span, format!("expected {expected}"))),
        ParseError::EmptyCond { span } => MietteDiagnostic::new("cond expects at least one clause")
            .with_code("stl::parse")
            .with_label(LabeledSpan::at(span, "empty cond")),
    };

    Report::new(diagnostic).with_source_code(NamedSource::new("repl", input.to_string()))
}

fn eval_error_report(error: EvalError) -> Report {
    Report::new(MietteDiagnostic::new(error.to_string()).with_code("stl::eval"))
}

struct LanguageValidator;

impl Validator for LanguageValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        if line.trim().is_empty() {
            return ValidationResult::Complete;
        }

        match parse(line) {
            Err(ParseError::UnexpectedEnd { .. }) => ValidationResult::Incomplete,
            _ => ValidationResult::Complete,
        }
    }
}
