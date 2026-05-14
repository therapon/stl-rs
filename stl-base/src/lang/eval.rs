use std::rc::Rc;

use crate::lang::{
    ast::{CondClause, Expr, LetBinding, Program},
    envs::{Env, EnvError},
    vals::{
        ExpVal, ExpValError, PrimitiveProc, ProcVal, bool_val, expval_to_bool, expval_to_num,
        expval_to_proc, num_val, proc_val,
    },
};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EvalError {
    #[error(transparent)]
    ExpVal(#[from] ExpValError),

    #[error(transparent)]
    Env(#[from] EnvError),

    #[error("{op} expects at least one argument")]
    EmptyArgs { op: &'static str },

    #[error("{op} expects {expected} argument(s), found {actual}")]
    WrongArity {
        op: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("no cond clause matched")]
    NoCondClauseMatched,
}

pub fn value_of_program(program: &Program) -> Result<ExpVal, EvalError> {
    value_of(&program.body, initial_env())
}

pub fn value_of(expr: &Expr, env: Rc<Env>) -> Result<ExpVal, EvalError> {
    match expr {
        Expr::ConstExp(n) => Ok(num_val(*n)),
        Expr::BoolExp(b) => Ok(bool_val(*b)),
        Expr::VarExp(var) => Ok(env.apply(var)?),
        Expr::FnExp { params, body } => Ok(proc_val(ProcVal::UserDefined {
            params: params.clone(),
            body: *body.clone(),
            saved_env: env,
        })),
        Expr::CallExp { operator, operands } => eval_call(operator, operands, env),
        Expr::OrExp(args) => eval_or(args, env),
        Expr::CondExp(clauses) => eval_cond(clauses, env),
        Expr::LetExp { bindings, body } => eval_let(bindings, body, env),
    }
}

fn initial_env() -> Rc<Env> {
    Env::empty()
        .extend("+", proc_val(ProcVal::Primitive(PrimitiveProc::Add)))
        .extend("-", proc_val(ProcVal::Primitive(PrimitiveProc::Sub)))
        .extend("not", proc_val(ProcVal::Primitive(PrimitiveProc::Not)))
}

fn eval_call(operator: &Expr, operands: &[Expr], env: Rc<Env>) -> Result<ExpVal, EvalError> {
    let proc = expval_to_proc(&value_of(operator, env.clone())?)?;
    let args = operands
        .iter()
        .map(|operand| value_of(operand, env.clone()))
        .collect::<Result<Vec<_>, _>>()?;

    apply_procedure(&proc, args)
}

fn apply_procedure(proc: &ProcVal, args: Vec<ExpVal>) -> Result<ExpVal, EvalError> {
    match proc {
        ProcVal::UserDefined {
            params,
            body,
            saved_env,
        } => {
            if params.len() != args.len() {
                return Err(EvalError::WrongArity {
                    op: "function",
                    expected: params.len(),
                    actual: args.len(),
                });
            }

            let mut call_env = saved_env.clone();

            for (param, arg) in params.iter().zip(args) {
                call_env = call_env.extend(param.clone(), arg);
            }

            value_of(body, call_env)
        }
        ProcVal::Primitive(primitive) => apply_primitive(primitive, &args),
    }
}

fn apply_primitive(primitive: &PrimitiveProc, args: &[ExpVal]) -> Result<ExpVal, EvalError> {
    match primitive {
        PrimitiveProc::Add => eval_add(args),
        PrimitiveProc::Sub => eval_sub(args),
        PrimitiveProc::Not => eval_not(args),
    }
}

fn eval_add(args: &[ExpVal]) -> Result<ExpVal, EvalError> {
    let sum = args
        .iter()
        .map(expval_to_num)
        .try_fold(0, |acc, n| -> Result<i64, EvalError> { Ok(acc + n?) })?;

    Ok(num_val(sum))
}

fn eval_sub(args: &[ExpVal]) -> Result<ExpVal, EvalError> {
    let Some((first, rest)) = args.split_first() else {
        return Err(EvalError::EmptyArgs { op: "-" });
    };

    let first = expval_to_num(first)?;

    if rest.is_empty() {
        return Ok(num_val(-first));
    }

    let difference = rest
        .iter()
        .map(expval_to_num)
        .try_fold(first, |acc, n| -> Result<i64, EvalError> { Ok(acc - n?) })?;

    Ok(num_val(difference))
}

fn eval_not(args: &[ExpVal]) -> Result<ExpVal, EvalError> {
    if args.len() != 1 {
        return Err(EvalError::WrongArity {
            op: "not",
            expected: 1,
            actual: args.len(),
        });
    }

    Ok(bool_val(!expval_to_bool(&args[0])?))
}

fn eval_or(args: &[Expr], env: Rc<Env>) -> Result<ExpVal, EvalError> {
    for arg in args {
        if value_of_expr_as_bool(arg, env.clone())? {
            return Ok(bool_val(true));
        }
    }

    Ok(bool_val(false))
}

fn eval_cond(clauses: &[CondClause], env: Rc<Env>) -> Result<ExpVal, EvalError> {
    for clause in clauses {
        if value_of_expr_as_bool(&clause.condition, env.clone())? {
            return value_of(&clause.result, env);
        }
    }

    Err(EvalError::NoCondClauseMatched)
}

fn eval_let(bindings: &[LetBinding], body: &Expr, env: Rc<Env>) -> Result<ExpVal, EvalError> {
    let mut extended_env = env.clone();

    for binding in bindings {
        let val = value_of(&binding.expr, env.clone())?;
        extended_env = extended_env.extend(binding.var.clone(), val);
    }

    value_of(body, extended_env)
}

fn value_of_expr_as_bool(expr: &Expr, env: Rc<Env>) -> Result<bool, EvalError> {
    let val = value_of(expr, env)?;
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
    fn eval_not_rejects_wrong_arity() {
        assert_eq!(
            eval("not(true false)").unwrap_err(),
            EvalError::WrongArity {
                op: "not",
                expected: 1,
                actual: 2
            }
        );
    }

    #[test]
    fn eval_primitive_procs_are_initial_bindings() {
        assert_eq!(eval("let (add = +) add(1 2)").unwrap(), num_val(3));
    }

    #[test]
    fn eval_function_expression_as_closure() {
        assert_eq!(
            eval("let (add1 = fn(x) +(x 1)) add1(41)").unwrap(),
            num_val(42)
        );
    }

    #[test]
    fn eval_function_captures_definition_environment() {
        assert_eq!(
            eval(
                "let (x = 10)
                    let (addx = fn(y) +(x y))
                        let (x = 100) addx(1)"
            )
            .unwrap(),
            num_val(11)
        );
    }

    #[test]
    fn eval_function_rejects_wrong_arity() {
        assert_eq!(
            eval("let (f = fn(x y) +(x y)) f(1)").unwrap_err(),
            EvalError::WrongArity {
                op: "function",
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn eval_call_rejects_non_procedures() {
        assert_eq!(
            eval("let (x = 1) x(2)").unwrap_err(),
            EvalError::ExpVal(ExpValError::ProcValExpected { actual: num_val(1) })
        );
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
    fn eval_let_extends_environment_for_body() {
        assert_eq!(
            eval(
                "let (
                    x = 1
                    y = +(2 3)
                ) +(x y)"
            )
            .unwrap(),
            num_val(6)
        );
    }

    #[test]
    fn eval_let_bindings_are_parallel() {
        assert_eq!(
            eval(
                "let (x = 1)
                    let (
                        x = 2
                        y = x
                    ) y"
            )
            .unwrap(),
            num_val(1)
        );
    }

    #[test]
    fn eval_let_body_uses_nearest_binding() {
        assert_eq!(
            eval(
                "let (x = 1)
                    let (x = 2) x"
            )
            .unwrap(),
            num_val(2)
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

    #[test]
    fn eval_rejects_unbound_variables() {
        assert_eq!(
            eval("x").unwrap_err(),
            EvalError::Env(EnvError::UnboundVariable {
                var: "x".to_string()
            })
        );
    }
}
