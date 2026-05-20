use stl_base::{
    lang::vals::ExpVal,
    testing::{check, error, value},
};

#[test]
fn simple_let() {
    check(
        r#"
        ; comments run to the end of the line
        let (
            x = 3
        ) x ; trailing comments are ignored too
        "#,
        value(ExpVal::num(3)),
    );
}

#[test]
fn unbound_variable_errors() {
    check(
        r#"
        foo
        "#,
        error(),
    );
}
