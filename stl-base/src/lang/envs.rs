use std::rc::Rc;

use crate::lang::vals::ExpVal;

pub type Symbol = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Env {
    EmptyEnv,
    ExtendEnv {
        var: Symbol,
        val: ExpVal,
        outer: Rc<Env>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EnvError {
    #[error("unbound variable: {var}")]
    UnboundVariable { var: Symbol },
}

pub fn empty_env() -> Rc<Env> {
    Rc::new(Env::EmptyEnv)
}

pub fn extend_env(var: impl Into<Symbol>, val: ExpVal, outer: Rc<Env>) -> Rc<Env> {
    Rc::new(Env::ExtendEnv {
        var: var.into(),
        val,
        outer,
    })
}

pub fn apply_env(env: &Env, search_var: &str) -> Result<ExpVal, EnvError> {
    match env {
        Env::EmptyEnv => Err(EnvError::UnboundVariable {
            var: search_var.to_string(),
        }),
        Env::ExtendEnv { var, val, outer } => {
            if search_var == var {
                Ok(val.clone())
            } else {
                apply_env(outer, search_var)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::vals::{bool_val, num_val};

    #[test]
    fn empty_env_has_no_bindings() {
        assert_eq!(
            apply_env(&empty_env(), "x").unwrap_err(),
            EnvError::UnboundVariable {
                var: "x".to_string()
            }
        );
    }

    #[test]
    fn apply_env_finds_nearest_binding() {
        let env = empty_env();
        let env = extend_env("x", num_val(10), env);
        let env = extend_env("x", bool_val(true), env);

        assert_eq!(apply_env(&env, "x").unwrap(), bool_val(true));
    }

    #[test]
    fn apply_env_searches_saved_environment() {
        let env = empty_env();
        let env = extend_env("x", num_val(10), env);
        let env = extend_env("y", bool_val(false), env);

        assert_eq!(apply_env(&env, "x").unwrap(), num_val(10));
        assert_eq!(apply_env(&env, "y").unwrap(), bool_val(false));
    }
}
