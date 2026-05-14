use crate::lang::{
    ast::{CondClause, Expr, Program},
    vals::{ExpVal, ExpValError, bool_val, expval_to_bool, expval_to_num, num_val},
};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EvalError {
    #[error(transparent)]
    ExpVal(#[from] ExpValError),

    #[error("{op} expects at least one argument")]
    EmptyArgs { op: &'static str },

    #[error("no cond clause matched")]
    NoCondClauseMatched,
}

pub fn value_of_program(program: &Program) -> Result<ExpVal, EvalError> {
    value_of(&program.body)
}

pub fn value_of(expr: &Expr) -> Result<ExpVal, EvalError> {
    match expr {
        Expr::ConstExp(n) => Ok(num_val(*n)),
        Expr::BoolExp(b) => Ok(bool_val(*b)),
        Expr::AddExp(args) => eval_add(args),
        Expr::SubExp(args) => eval_sub(args),
        Expr::OrExp(args) => eval_or(args),
        Expr::NotExp(arg) => eval_not(arg),
        Expr::CondExp(clauses) => eval_cond(clauses),
    }
}

fn eval_add(args: &[Expr]) -> Result<ExpVal, EvalError> {
    let sum = args
        .iter()
        .map(value_of_num)
        .try_fold(0, |acc, n| -> Result<i64, EvalError> { Ok(acc + n?) })?;

    Ok(num_val(sum))
}

fn eval_sub(args: &[Expr]) -> Result<ExpVal, EvalError> {
    let Some((first, rest)) = args.split_first() else {
        return Err(EvalError::EmptyArgs { op: "-" });
    };

    let first = value_of_num(first)?;

    if rest.is_empty() {
        return Ok(num_val(-first));
    }

    let difference = rest
        .iter()
        .map(value_of_num)
        .try_fold(first, |acc, n| -> Result<i64, EvalError> { Ok(acc - n?) })?;

    Ok(num_val(difference))
}

fn eval_or(args: &[Expr]) -> Result<ExpVal, EvalError> {
    for arg in args {
        if value_of_bool(arg)? {
            return Ok(bool_val(true));
        }
    }

    Ok(bool_val(false))
}

fn eval_not(arg: &Expr) -> Result<ExpVal, EvalError> {
    Ok(bool_val(!value_of_bool(arg)?))
}

fn eval_cond(clauses: &[CondClause]) -> Result<ExpVal, EvalError> {
    for clause in clauses {
        if value_of_bool(&clause.condition)? {
            return value_of(&clause.result);
        }
    }

    Err(EvalError::NoCondClauseMatched)
}

fn value_of_num(expr: &Expr) -> Result<i64, EvalError> {
    let val = value_of(expr)?;
    Ok(expval_to_num(&val)?)
}

fn value_of_bool(expr: &Expr) -> Result<bool, EvalError> {
    let val = value_of(expr)?;
    Ok(expval_to_bool(&val)?)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::{parser::parse, vals::ExpValError};

    fn eval(input: &str) -> Result<ExpVal, EvalError> {
        value_of_program(&parse(input).unwrap())
    }

    #[test]
    fn eval_adds_zero_or_more_numbers() {
        assert_eq!(eval("+()").unwrap(), num_val(0));
        assert_eq!(eval("+(1)").unwrap(), num_val(1));
        assert_eq!(eval("+(1 2 3)").unwrap(), num_val(6));
    }

    #[test]
    fn eval_subtracts_like_lisp() {
        assert_eq!(eval("-(10)").unwrap(), num_val(-10));
        assert_eq!(eval("-(10 3)").unwrap(), num_val(7));
        assert_eq!(eval("-(10 3 2)").unwrap(), num_val(5));
    }

    #[test]
    fn eval_rejects_empty_subtraction() {
        assert_eq!(eval("-()").unwrap_err(), EvalError::EmptyArgs { op: "-" });
    }

    #[test]
    fn eval_or_short_circuits() {
        assert_eq!(eval("or()").unwrap(), bool_val(false));
        assert_eq!(eval("or(false false)").unwrap(), bool_val(false));
        assert_eq!(eval("or(false true)").unwrap(), bool_val(true));
        assert_eq!(eval("or(true +(1 true))").unwrap(), bool_val(true));
    }

    #[test]
    fn eval_not_negates_boolean() {
        assert_eq!(eval("not(true)").unwrap(), bool_val(false));
        assert_eq!(eval("not(false)").unwrap(), bool_val(true));
    }

    #[test]
    fn eval_cond_uses_first_matching_clause() {
        assert_eq!(
            eval(
                "cond (
                    false => +(1 true)
                    true => +(2 3)
                    true => +(1 false)
                )"
            )
            .unwrap(),
            num_val(5)
        );
    }

    #[test]
    fn eval_rejects_unmatched_cond() {
        assert_eq!(
            eval("cond (false => 1)").unwrap_err(),
            EvalError::NoCondClauseMatched
        );
    }

    #[test]
    fn eval_rejects_wrong_runtime_types() {
        assert_eq!(
            eval("+(1 true)").unwrap_err(),
            EvalError::ExpVal(ExpValError::NumValExpected {
                actual: bool_val(true)
            })
        );
    }
}
