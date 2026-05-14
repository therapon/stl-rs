mod lang;
mod repl;

fn main() -> miette::Result<()> {
    repl::run()
}
