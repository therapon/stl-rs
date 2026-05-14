use std::rc::Rc;

use crate::lang::{
    ast::{CondClause, Expr, LetBinding, LetRecBinding, Program},
    envs::{Env, EnvError},
    vals::{ExpVal, ExpValError, PrimitiveProc, ProcVal},
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

#[derive(Clone, Debug)]
pub struct Evaluator {
    initial_env: Rc<Env>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            initial_env: Self::initial_env(),
        }
    }

    pub fn with_initial_env(initial_env: Rc<Env>) -> Self {
        Self { initial_env }
    }

    pub fn eval_program(&self, program: &Program) -> Result<ExpVal, EvalError> {
        self.eval_expr(&program.body)
    }

    pub fn eval_expr(&self, expr: &Expr) -> Result<ExpVal, EvalError> {
        self.eval_expr_in_env(expr, self.initial_env.clone())
    }

    fn initial_env() -> Rc<Env> {
        Env::empty()
            .extend("+", ExpVal::proc(ProcVal::Primitive(PrimitiveProc::Add)))
            .extend("-", ExpVal::proc(ProcVal::Primitive(PrimitiveProc::Sub)))
            .extend("not", ExpVal::proc(ProcVal::Primitive(PrimitiveProc::Not)))
    }

    fn eval_expr_in_env(&self, expr: &Expr, env: Rc<Env>) -> Result<ExpVal, EvalError> {
        match expr {
            Expr::ConstExp(n) => Ok(ExpVal::num(*n)),
            Expr::BoolExp(b) => Ok(ExpVal::boolean(*b)),
            Expr::VarExp(var) => Ok(env.apply(var)?),
            Expr::FnExp { params, body } => Ok(ExpVal::proc(ProcVal::UserDefined {
                params: params.clone(),
                body: *body.clone(),
                saved_env: env,
            })),
            Expr::CallExp { operator, operands } => self.eval_call(operator, operands, env),
            Expr::OrExp(args) => self.eval_or(args, env),
            Expr::CondExp(clauses) => self.eval_cond(clauses, env),
            Expr::LetExp { bindings, body } => self.eval_let(bindings, body, env),
            Expr::LetRecExp { bindings, body } => self.eval_letrec(bindings, body, env),
        }
    }

    fn eval_call(
        &self,
        operator: &Expr,
        operands: &[Expr],
        env: Rc<Env>,
    ) -> Result<ExpVal, EvalError> {
        let proc = self.eval_expr_in_env(operator, env.clone())?.as_proc()?;
        let args = operands
            .iter()
            .map(|operand| self.eval_expr_in_env(operand, env.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        self.apply_procedure(&proc, args)
    }

    fn apply_procedure(&self, proc: &ProcVal, args: Vec<ExpVal>) -> Result<ExpVal, EvalError> {
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

                self.eval_expr_in_env(body, call_env)
            }
            ProcVal::Primitive(primitive) => self.apply_primitive(primitive, &args),
        }
    }

    fn apply_primitive(
        &self,
        primitive: &PrimitiveProc,
        args: &[ExpVal],
    ) -> Result<ExpVal, EvalError> {
        match primitive {
            PrimitiveProc::Add => self.eval_add(args),
            PrimitiveProc::Sub => self.eval_sub(args),
            PrimitiveProc::Not => self.eval_not(args),
        }
    }

    fn eval_add(&self, args: &[ExpVal]) -> Result<ExpVal, EvalError> {
        let sum = args
            .iter()
            .map(ExpVal::as_num)
            .try_fold(0, |acc, n| -> Result<i64, EvalError> { Ok(acc + n?) })?;

        Ok(ExpVal::num(sum))
    }

    fn eval_sub(&self, args: &[ExpVal]) -> Result<ExpVal, EvalError> {
        let Some((first, rest)) = args.split_first() else {
            return Err(EvalError::EmptyArgs { op: "-" });
        };

        let first = first.as_num()?;

        if rest.is_empty() {
            return Ok(ExpVal::num(-first));
        }

        let difference = rest
            .iter()
            .map(ExpVal::as_num)
            .try_fold(first, |acc, n| -> Result<i64, EvalError> { Ok(acc - n?) })?;

        Ok(ExpVal::num(difference))
    }

    fn eval_not(&self, args: &[ExpVal]) -> Result<ExpVal, EvalError> {
        if args.len() != 1 {
            return Err(EvalError::WrongArity {
                op: "not",
                expected: 1,
                actual: args.len(),
            });
        }

        Ok(ExpVal::boolean(!args[0].as_bool()?))
    }

    fn eval_or(&self, args: &[Expr], env: Rc<Env>) -> Result<ExpVal, EvalError> {
        for arg in args {
            if self.eval_expr_as_bool(arg, env.clone())? {
                return Ok(ExpVal::boolean(true));
            }
        }

        Ok(ExpVal::boolean(false))
    }

    fn eval_cond(&self, clauses: &[CondClause], env: Rc<Env>) -> Result<ExpVal, EvalError> {
        for clause in clauses {
            if self.eval_expr_as_bool(&clause.condition, env.clone())? {
                return self.eval_expr_in_env(&clause.result, env);
            }
        }

        Err(EvalError::NoCondClauseMatched)
    }

    fn eval_let(
        &self,
        bindings: &[LetBinding],
        body: &Expr,
        env: Rc<Env>,
    ) -> Result<ExpVal, EvalError> {
        let mut extended_env = env.clone();

        for binding in bindings {
            let val = self.eval_expr_in_env(&binding.expr, env.clone())?;
            extended_env = extended_env.extend(binding.var.clone(), val);
        }

        self.eval_expr_in_env(body, extended_env)
    }

    fn eval_letrec(
        &self,
        bindings: &[LetRecBinding],
        body: &Expr,
        env: Rc<Env>,
    ) -> Result<ExpVal, EvalError> {
        self.eval_expr_in_env(body, env.extend_rec(bindings.to_vec()))
    }

    fn eval_expr_as_bool(&self, expr: &Expr, env: Rc<Env>) -> Result<bool, EvalError> {
        let val = self.eval_expr_in_env(expr, env)?;
        Ok(val.as_bool()?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::{envs::Env, parser::parse, vals::ExpValError};

    fn eval(input: &str) -> Result<ExpVal, EvalError> {
        Evaluator::new().eval_program(&parse(input).unwrap())
    }

    #[test]
    fn evaluator_accepts_custom_initial_environment() {
        let evaluator = Evaluator::with_initial_env(Env::empty().extend("x", ExpVal::num(5)));

        assert_eq!(
            evaluator.eval_program(&parse("x").unwrap()).unwrap(),
            ExpVal::num(5)
        );
    }

    #[test]
    fn eval_adds_zero_or_more_numbers() {
        assert_eq!(eval("+()").unwrap(), ExpVal::num(0));
        assert_eq!(eval("+(1)").unwrap(), ExpVal::num(1));
        assert_eq!(eval("+(1 2 3)").unwrap(), ExpVal::num(6));
    }

    #[test]
    fn eval_subtracts_like_lisp() {
        assert_eq!(eval("-(10)").unwrap(), ExpVal::num(-10));
        assert_eq!(eval("-(10 3)").unwrap(), ExpVal::num(7));
        assert_eq!(eval("-(10 3 2)").unwrap(), ExpVal::num(5));
    }

    #[test]
    fn eval_rejects_empty_subtraction() {
        assert_eq!(eval("-()").unwrap_err(), EvalError::EmptyArgs { op: "-" });
    }

    #[test]
    fn eval_or_short_circuits() {
        assert_eq!(eval("or()").unwrap(), ExpVal::boolean(false));
        assert_eq!(eval("or(false false)").unwrap(), ExpVal::boolean(false));
        assert_eq!(eval("or(false true)").unwrap(), ExpVal::boolean(true));
        assert_eq!(eval("or(true +(1 true))").unwrap(), ExpVal::boolean(true));
    }

    #[test]
    fn eval_not_negates_boolean() {
        assert_eq!(eval("not(true)").unwrap(), ExpVal::boolean(false));
        assert_eq!(eval("not(false)").unwrap(), ExpVal::boolean(true));
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
        assert_eq!(eval("let (add = +) add(1 2)").unwrap(), ExpVal::num(3));
    }

    #[test]
    fn eval_function_expression_as_closure() {
        assert_eq!(
            eval("let (add1 = fn(x) +(x 1)) add1(41)").unwrap(),
            ExpVal::num(42)
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
            ExpVal::num(11)
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
            EvalError::ExpVal(ExpValError::ProcValExpected {
                actual: ExpVal::num(1)
            })
        );
    }

    #[test]
    fn eval_letrec_binds_recursive_function() {
        assert_eq!(
            eval(
                "letrec (
                    f = fn(x) +(x 1)
                ) f(41)"
            )
            .unwrap(),
            ExpVal::num(42)
        );
    }

    #[test]
    fn eval_letrec_function_can_look_up_itself_late() {
        assert_eq!(
            eval(
                "letrec (
                    f = fn(x) let (self = f) x
                ) f(41)"
            )
            .unwrap(),
            ExpVal::num(41)
        );
    }

    #[test]
    fn eval_letrec_supports_mutual_function_lookup() {
        assert_eq!(
            eval(
                "letrec (
                    f = fn(x) g(x)
                    g = fn(y) +(y 1)
                ) f(41)"
            )
            .unwrap(),
            ExpVal::num(42)
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
            ExpVal::num(5)
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
            ExpVal::num(6)
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
            ExpVal::num(1)
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
            ExpVal::num(2)
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
                actual: ExpVal::boolean(true)
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
