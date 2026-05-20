use std::rc::Rc;

use crate::lang::{
    ast::LetRecBinding,
    vals::{ExpVal, Procedure},
};

pub type Symbol = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Env {
    EmptyEnv,
    ExtendEnv {
        var: Symbol,
        val: ExpVal,
        outer: Rc<Env>,
    },
    ExtendRecEnv {
        bindings: Vec<LetRecBinding>,
        outer: Rc<Env>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EnvError {
    #[error("unbound variable: {var}")]
    UnboundVariable { var: Symbol },
}

impl Env {
    pub fn empty() -> Rc<Self> {
        Rc::new(Self::EmptyEnv)
    }

    pub fn extend(self: Rc<Self>, var: impl Into<Symbol>, val: ExpVal) -> Rc<Self> {
        Rc::new(Self::ExtendEnv {
            var: var.into(),
            val,
            outer: self,
        })
    }

    pub fn extend_rec(self: Rc<Self>, bindings: Vec<LetRecBinding>) -> Rc<Self> {
        Rc::new(Self::ExtendRecEnv {
            bindings,
            outer: self,
        })
    }

    pub fn apply(&self, search_var: &str) -> Result<ExpVal, EnvError> {
        match self {
            Self::EmptyEnv => Err(EnvError::UnboundVariable {
                var: search_var.to_string(),
            }),
            Self::ExtendEnv { var, val, outer } => {
                if search_var == var {
                    Ok(val.clone())
                } else {
                    outer.apply(search_var)
                }
            }
            Self::ExtendRecEnv { bindings, outer } => {
                if let Some(binding) = bindings.iter().find(|binding| binding.name == search_var) {
                    Ok(ExpVal::proc(Procedure::Closure {
                        params: binding.params.clone(),
                        body: binding.body.clone(),
                        saved_env: Rc::new(self.clone()),
                    }))
                } else {
                    outer.apply(search_var)
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_env_has_no_bindings() {
        assert_eq!(
            Env::empty().apply("x").unwrap_err(),
            EnvError::UnboundVariable {
                var: "x".to_string()
            }
        );
    }

    #[test]
    fn apply_env_finds_nearest_binding() {
        let env = Env::empty()
            .extend("x", ExpVal::num(10))
            .extend("x", ExpVal::boolean(true));

        assert_eq!(env.apply("x").unwrap(), ExpVal::boolean(true));
    }

    #[test]
    fn apply_env_searches_saved_environment() {
        let env = Env::empty()
            .extend("x", ExpVal::num(10))
            .extend("y", ExpVal::boolean(false));

        assert_eq!(env.apply("x").unwrap(), ExpVal::num(10));
        assert_eq!(env.apply("y").unwrap(), ExpVal::boolean(false));
    }

    #[test]
    fn apply_env_builds_late_bound_recursive_proc() {
        let env = Env::empty().extend_rec(vec![LetRecBinding::new(
            "f",
            vec!["x".to_string()],
            crate::lang::ast::Expr::VarExp("x".to_string()),
        )]);

        assert!(matches!(
            env.apply("f").unwrap(),
            ExpVal::ProcVal(Procedure::Closure { .. })
        ));
    }
}
