use std::{
    env, fs,
    path::Path,
    path::PathBuf,
    process::{Command, Stdio},
};

use miette::{IntoDiagnostic, LabeledSpan, MietteDiagnostic, NamedSource, Report, Result};
use reedline::{
    ColumnarMenu, Completer, DefaultCompleter, DefaultPrompt, DefaultPromptSegment, Emacs, KeyCode,
    KeyModifiers, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal, Span, Suggestion,
    ValidationResult, Validator, default_emacs_keybindings,
};

use crate::lang::{
    eval::{EvalError, Evaluator},
    parser::{ParseError, parse, parse_programs},
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

        if let Some(command) = MetaCommand::parse(trimmed)? {
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
            MetaCommand::Load(path) => {
                self.load_file(&path)?;
                Ok(ReplControl::Continue)
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

    fn load_file(&self, path: &Path) -> Result<()> {
        let source = fs::read_to_string(path).into_diagnostic()?;
        let source_name = path.display().to_string();

        let programs = match parse_programs(&source) {
            Ok(programs) => programs,
            Err(error) => {
                self.print_report(self.parse_error_report(source_name, &source, error));
                return Ok(());
            }
        };

        for program in programs {
            match self.evaluator.eval_program(&program) {
                Ok(val) => println!("{val}"),
                Err(error) => {
                    self.print_report(self.eval_error_report(error));
                    break;
                }
            }
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
            Err(error) => self.print_report(self.parse_error_report("repl", input, error)),
        }
    }

    fn print_report(&self, report: Report) {
        eprintln!("{report:?}");
    }

    fn parse_error_report(
        &self,
        source_name: impl Into<String>,
        input: &str,
        error: ParseError,
    ) -> Report {
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

        let source_name = source_name.into();
        Report::new(diagnostic).with_source_code(NamedSource::new(source_name, input.to_string()))
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum MetaCommand {
    Quit,
    Rebuild,
    Load(PathBuf),
}

impl MetaCommand {
    fn parse(input: &str) -> Result<Option<Self>> {
        let (command, arg) = input
            .split_once(char::is_whitespace)
            .map_or((input, None), |(command, arg)| {
                (command, Some(arg.trim()).filter(|arg| !arg.is_empty()))
            });

        match command {
            ":quit" => {
                reject_arg(":quit", arg)?;
                Ok(Some(Self::Quit))
            }
            ":rebuild" => {
                reject_arg(":rebuild", arg)?;
                Ok(Some(Self::Rebuild))
            }
            ":load" => {
                let Some(path) = arg else {
                    return Err(miette::miette!("usage: :load <path>"));
                };

                Ok(Some(Self::Load(expand_path(unquote_path_arg(path)))))
            }
            _ => Ok(None),
        }
    }
}

fn reject_arg(command: &str, arg: Option<&str>) -> Result<()> {
    if arg.is_some() {
        return Err(miette::miette!("usage: {command}"));
    }

    Ok(())
}

fn unquote_path_arg(path: &str) -> &str {
    if path.len() < 2 {
        return path;
    }

    let quoted_with_double_quotes = path.starts_with('"') && path.ends_with('"');
    let quoted_with_single_quotes = path.starts_with('\'') && path.ends_with('\'');

    if quoted_with_double_quotes || quoted_with_single_quotes {
        &path[1..path.len() - 1]
    } else {
        path
    }
}

fn expand_path(path: &str) -> PathBuf {
    if path == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = home_dir()
    {
        return home.join(rest);
    }

    PathBuf::from(path)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

const COMPLETION_MENU: &str = "completion_menu";

fn line_editor() -> Reedline {
    let completer = Box::new(ReplCompleter::new());
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

struct ReplCompleter {
    language_completer: DefaultCompleter,
}

impl ReplCompleter {
    fn new() -> Self {
        Self {
            language_completer: language_completer(),
        }
    }
}

impl Completer for ReplCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let Some(prefix) = line.get(..pos) else {
            return Vec::new();
        };

        if let Some(path_start) = load_path_start(prefix) {
            complete_load_path(prefix, pos, path_start)
        } else {
            self.language_completer.complete(line, pos)
        }
    }
}

fn language_completer() -> DefaultCompleter {
    let mut completer =
        DefaultCompleter::with_inclusions(&[':', '+', '-', '=', '(', ')']).set_min_word_len(1);
    completer.insert(completion_words());
    completer
}

fn load_path_start(input: &str) -> Option<usize> {
    let leading_whitespace = input.len() - input.trim_start().len();
    let input = &input[leading_whitespace..];
    let after_command = input.strip_prefix(":load")?;

    if after_command.is_empty() {
        return None;
    }

    let whitespace_len = after_command.len() - after_command.trim_start().len();

    if whitespace_len == 0 {
        return None;
    }

    Some(leading_whitespace + ":load".len() + whitespace_len)
}

fn complete_load_path(input: &str, pos: usize, path_start: usize) -> Vec<Suggestion> {
    let path_arg = &input[path_start..pos];
    let (path_text, replacement_start) = if path_arg.starts_with('"') || path_arg.starts_with('\'')
    {
        (&path_arg[1..], path_start + 1)
    } else {
        (path_arg, path_start)
    };

    let (dir_prefix, file_prefix) = split_path_for_completion(path_text);
    let lookup_dir = if dir_prefix.is_empty() {
        PathBuf::from(".")
    } else {
        expand_path(dir_prefix)
    };

    let Ok(entries) = fs::read_dir(lookup_dir) else {
        return Vec::new();
    };

    let mut suggestions = entries
        .filter_map(|entry| path_suggestion(entry.ok()?, dir_prefix, file_prefix))
        .map(|(value, description)| Suggestion {
            value,
            description: Some(description),
            span: Span::new(replacement_start, pos),
            append_whitespace: false,
            ..Default::default()
        })
        .collect::<Vec<_>>();

    suggestions.sort_by(|left, right| left.value.cmp(&right.value));
    suggestions
}

fn split_path_for_completion(path: &str) -> (&str, &str) {
    path.rfind('/')
        .map_or(("", path), |index| path.split_at(index + 1))
}

fn path_suggestion(
    entry: fs::DirEntry,
    dir_prefix: &str,
    file_prefix: &str,
) -> Option<(String, String)> {
    let file_name = entry.file_name().into_string().ok()?;

    if !file_name.starts_with(file_prefix) {
        return None;
    }

    let file_type = entry.file_type().ok()?;
    let is_dir = file_type.is_dir();
    let suffix = if is_dir { "/" } else { "" };
    let description = if is_dir { "directory" } else { "file" };

    Some((
        format!("{dir_prefix}{file_name}{suffix}"),
        description.to_string(),
    ))
}

fn completion_words() -> Vec<String> {
    [
        ":quit", ":rebuild", ":load", ":load ", "+(", "-(", "=(", "not(", "or(", "cond", "cond (",
        "let", "let (", "letrec", "letrec (", "fn", "fn(", "true", "false",
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
        assert_eq!(
            MetaCommand::parse(":quit").unwrap(),
            Some(MetaCommand::Quit)
        );
        assert_eq!(
            MetaCommand::parse(":rebuild").unwrap(),
            Some(MetaCommand::Rebuild)
        );
        assert_eq!(
            MetaCommand::parse(":load examples.stl").unwrap(),
            Some(MetaCommand::Load(PathBuf::from("examples.stl")))
        );
        assert_eq!(
            MetaCommand::parse(":load 'some file.stl'").unwrap(),
            Some(MetaCommand::Load(PathBuf::from("some file.stl")))
        );
        assert_eq!(MetaCommand::parse("+(1 2)").unwrap(), None);
        assert_eq!(MetaCommand::parse(":unknown").unwrap(), None);
    }

    #[test]
    fn parse_meta_commands_reject_bad_usage() {
        assert!(MetaCommand::parse(":load").is_err());
        assert!(MetaCommand::parse(":quit now").is_err());
    }

    #[test]
    fn load_path_start_finds_path_after_load_command() {
        assert_eq!(load_path_start(":load "), Some(6));
        assert_eq!(load_path_start("  :load foo"), Some(8));
        assert_eq!(load_path_start(":load"), None);
        assert_eq!(load_path_start(":rebuild"), None);
    }

    #[test]
    fn split_path_for_completion_separates_directory_prefix() {
        assert_eq!(split_path_for_completion("foo"), ("", "foo"));
        assert_eq!(split_path_for_completion("dir/foo"), ("dir/", "foo"));
        assert_eq!(split_path_for_completion("dir/"), ("dir/", ""));
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
        assert!(words.contains(&":load".to_string()));
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

    #[test]
    fn repl_completer_completes_load_paths() {
        let dir = test_dir("repl-load-complete");
        let nested_dir = dir.join("nested");
        fs::create_dir_all(&nested_dir).unwrap();
        fs::write(dir.join("program.stl"), "+(1 2)").unwrap();

        let mut completer = ReplCompleter::new();
        let input = format!(":load {}/p", dir.display());

        let values = completer
            .complete(&input, input.len())
            .into_iter()
            .map(|suggestion| suggestion.value)
            .collect::<Vec<_>>();

        assert!(values.contains(&format!("{}/program.stl", dir.display())));

        let input = format!(":load {}/n", dir.display());
        let values = completer
            .complete(&input, input.len())
            .into_iter()
            .map(|suggestion| suggestion.value)
            .collect::<Vec<_>>();

        assert!(values.contains(&format!("{}/nested/", dir.display())));

        fs::remove_dir_all(dir).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("stl-rs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
