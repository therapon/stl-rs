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
            Env::empty().apply("x").unwrap_err(),
            EnvError::UnboundVariable {
                var: "x".to_string()
            }
        );
    }

    #[test]
    fn apply_env_finds_nearest_binding() {
        let env = Env::empty()
            .extend("x", num_val(10))
            .extend("x", bool_val(true));

        assert_eq!(env.apply("x").unwrap(), bool_val(true));
    }

    #[test]
    fn apply_env_searches_saved_environment() {
        let env = Env::empty()
            .extend("x", num_val(10))
            .extend("y", bool_val(false));

        assert_eq!(env.apply("x").unwrap(), num_val(10));
        assert_eq!(env.apply("y").unwrap(), bool_val(false));
    }
}
