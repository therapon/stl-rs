use stl_base::{
    lang::vals::ExpVal,
    testing::{check, error, value},
};

#[test]
fn simple_let() {
    check(
        r#"
        let (
            x = 3
        ) x
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
