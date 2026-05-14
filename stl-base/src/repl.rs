use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

use miette::{IntoDiagnostic, LabeledSpan, MietteDiagnostic, NamedSource, Report, Result};
use reedline::{
    ColumnarMenu, DefaultCompleter, DefaultPrompt, DefaultPromptSegment, Emacs, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal, ValidationResult,
    Validator, default_emacs_keybindings,
};

use crate::lang::{
    eval::{EvalError, Evaluator},
    parser::{ParseError, parse},
    scanner::ScanError,
};

pub struct Repl {
    line_editor: Reedline,
    prompt: DefaultPrompt,
    evaluator: Evaluator,
}

impl Default for Repl {
    fn default() -> Self {
        Self::new()
    }
}

impl Repl {
    pub fn new() -> Self {
        Self {
            line_editor: line_editor(),
            prompt: DefaultPrompt::new(
                DefaultPromptSegment::Basic("stl".to_string()),
                DefaultPromptSegment::Empty,
            ),
            evaluator: Evaluator::new(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            match self.line_editor.read_line(&self.prompt).into_diagnostic()? {
                Signal::Success(input) => {
                    if self.handle_input(&input)? == ReplControl::Exit {
                        break;
                    }
                }
                Signal::CtrlD | Signal::CtrlC => break,
                signal => eprintln!("{signal:?}"),
            }
        }

        Ok(())
    }

    fn handle_input(&self, input: &str) -> Result<ReplControl> {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return Ok(ReplControl::Continue);
        }

        if let Some(command) = MetaCommand::parse(trimmed) {
            return self.handle_meta_command(command);
        }

        if trimmed.starts_with(':') {
            eprintln!("unknown meta command: {trimmed}");
            return Ok(ReplControl::Continue);
        }

        self.eval_and_print(input);
        Ok(ReplControl::Continue)
    }

    fn handle_meta_command(&self, command: MetaCommand) -> Result<ReplControl> {
        match command {
            MetaCommand::Quit => Ok(ReplControl::Exit),
            MetaCommand::Rebuild => {
                self.rebuild_and_restart()?;
                Ok(ReplControl::Exit)
            }
        }
    }

    fn rebuild_and_restart(&self) -> Result<()> {
        let manifest_path = manifest_path();

        let status = Command::new("cargo")
            .arg("build")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .status()
            .into_diagnostic()?;

        if !status.success() {
            return Err(miette::miette!("cargo build failed with status {status}"));
        }

        let current_exe = env::current_exe().into_diagnostic()?;

        let status = Command::new(current_exe)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .into_diagnostic()?;

        if !status.success() {
            return Err(miette::miette!(
                "restarted REPL exited with status {status}"
            ));
        }

        Ok(())
    }

    fn eval_and_print(&self, input: &str) {
        if input.trim().is_empty() {
            return;
        }

        match parse(input) {
            Ok(program) => match self.evaluator.eval_program(&program) {
                Ok(val) => println!("{val}"),
                Err(error) => self.print_report(self.eval_error_report(error)),
            },
            Err(error) => self.print_report(self.parse_error_report(input, error)),
        }
    }

    fn print_report(&self, report: Report) {
        eprintln!("{report:?}");
    }

    fn parse_error_report(&self, input: &str, error: ParseError) -> Report {
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
            ParseError::EmptyCond { span } => {
                MietteDiagnostic::new("cond expects at least one clause")
                    .with_code("stl::parse")
                    .with_label(LabeledSpan::at(span, "empty cond"))
            }
        };

        Report::new(diagnostic).with_source_code(NamedSource::new("repl", input.to_string()))
    }

    fn eval_error_report(&self, error: EvalError) -> Report {
        Report::new(MietteDiagnostic::new(error.to_string()).with_code("stl::eval"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReplControl {
    Continue,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetaCommand {
    Quit,
    Rebuild,
}

impl MetaCommand {
    fn parse(input: &str) -> Option<Self> {
        match input {
            ":quit" => Some(Self::Quit),
            ":rebuild" => Some(Self::Rebuild),
            _ => None,
        }
    }
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

const COMPLETION_MENU: &str = "completion_menu";

fn line_editor() -> Reedline {
    let completer = Box::new(language_completer());
    let completion_menu = Box::new(ColumnarMenu::default().with_name(COMPLETION_MENU));
    let mut keybindings = default_emacs_keybindings();

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu(COMPLETION_MENU.to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    Reedline::create()
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(Box::new(Emacs::new(keybindings)))
        .with_validator(Box::new(LanguageValidator))
}

fn language_completer() -> DefaultCompleter {
    let mut completer =
        DefaultCompleter::with_inclusions(&[':', '+', '-', '=', '(', ')']).set_min_word_len(1);
    completer.insert(completion_words());
    completer
}

fn completion_words() -> Vec<String> {
    [
        ":quit", ":rebuild", "+(", "-(", "=(", "not(", "or(", "cond", "cond (", "let", "let (",
        "letrec", "letrec (", "fn", "fn(", "true", "false",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
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

#[cfg(test)]
mod test {
    use super::*;
    use reedline::Completer;

    #[test]
    fn parse_meta_commands() {
        assert_eq!(MetaCommand::parse(":quit"), Some(MetaCommand::Quit));
        assert_eq!(MetaCommand::parse(":rebuild"), Some(MetaCommand::Rebuild));
        assert_eq!(MetaCommand::parse("+(1 2)"), None);
        assert_eq!(MetaCommand::parse(":unknown"), None);
    }

    #[test]
    fn finds_manifest_path() {
        assert!(manifest_path().ends_with("Cargo.toml"));
    }

    #[test]
    fn completion_words_include_meta_commands_and_language_forms() {
        let words = completion_words();

        assert!(words.contains(&":quit".to_string()));
        assert!(words.contains(&":rebuild".to_string()));
        assert!(words.contains(&"letrec".to_string()));
        assert!(words.contains(&"fn(".to_string()));
        assert!(words.contains(&"=(".to_string()));
    }

    #[test]
    fn language_completer_completes_meta_commands_and_forms() {
        let mut completer = language_completer();

        let meta_values = completer
            .complete(":r", 2)
            .into_iter()
            .map(|suggestion| suggestion.value)
            .collect::<Vec<_>>();
        assert!(meta_values.contains(&":rebuild".to_string()));

        let form_values = completer
            .complete("let", 3)
            .into_iter()
            .map(|suggestion| suggestion.value)
            .collect::<Vec<_>>();
        assert!(form_values.contains(&"letrec".to_string()));
    }
}
