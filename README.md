EOPL inspired languages in Rust. 


## Base language 

### Values 

* Ints 
* Booleans 
* functions 
    * Built in `+` `-` `=` `not` `or`
    * Closures with `fn(a ...) expr`


### Expressions 

* Constants 
* Variable refs: `x` `+` `check?`
* Procedure calls: <id>(<args>), e.g., `f(1 2)`, `=(1 1)`, `+(a b)`
* Anonymous functions: `fn(a b) +(a b)`
* `let` expressions: `let (x = 1 y = 2) +(x y)`
    * parallel `let` cannot depend on previous bindings in this same `let` 
* `letrec` expressions for recursive bindings: `letrec (f = fn(x) ..f(...)... ) f(10)`
* Conditional expression: `cond ( test => result ...)`
    * use `true => e` for default or else-branch

### Identifiers 

Scheme like, i.e., `zero?` `number->boolean` 


## REPL 

Meta commands: 

* `:quit` exit 
* `:rebuild` rebuilds Rust code and restarts REPL
* `:load <path>` loads a file (has some tab autocompletion)

File with code can have 1 or multiple top level expressions.
