mod lang;
mod repl;

fn main() -> miette::Result<()> {
    let mut repl = repl::Repl::new();
    repl.run()
}
