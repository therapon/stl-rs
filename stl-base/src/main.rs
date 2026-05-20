fn main() -> miette::Result<()> {
    let mut repl = stl_base::repl::Repl::new();
    repl.run()
}
