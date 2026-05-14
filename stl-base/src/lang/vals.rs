use std::fmt::Display;
use std::rc::Rc;

use crate::lang::{ast::Expr, envs::Env};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpVal {
    NumVal(i64),
    BoolVal(bool),
    ProcVal(ProcVal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProcVal {
    UserDefined {
        params: Vec<String>,
        body: Expr,
        saved_env: Rc<Env>,
    },
    Primitive(PrimitiveProc),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrimitiveProc {
    Add,
    Sub,
    Not,
}

impl Display for ExpVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpVal::NumVal(n) => write!(f, "{n}"),
            ExpVal::BoolVal(b) => write!(f, "{b}"),
            ExpVal::ProcVal(_) => write!(f, "<procedure>"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ExpValError {
    #[error("expected number, found {actual}")]
    NumValExpected { actual: ExpVal },

    #[error("expected boolean, found {actual}")]
    BoolValExpected { actual: ExpVal },

    #[error("expected procedure, found {actual}")]
    ProcValExpected { actual: ExpVal },
}

impl ExpVal {
    pub fn num(n: i64) -> Self {
        Self::NumVal(n)
    }

    pub fn boolean(b: bool) -> Self {
        Self::BoolVal(b)
    }

    pub fn proc(proc: ProcVal) -> Self {
        Self::ProcVal(proc)
    }

    pub fn as_num(&self) -> Result<i64, ExpValError> {
        match self {
            Self::NumVal(n) => Ok(*n),
            _ => Err(ExpValError::NumValExpected {
                actual: self.clone(),
            }),
        }
    }

    pub fn as_bool(&self) -> Result<bool, ExpValError> {
        match self {
            Self::BoolVal(b) => Ok(*b),
            _ => Err(ExpValError::BoolValExpected {
                actual: self.clone(),
            }),
        }
    }

    pub fn as_proc(&self) -> Result<ProcVal, ExpValError> {
        match self {
            Self::ProcVal(proc) => Ok(proc.clone()),
            _ => Err(ExpValError::ProcValExpected {
                actual: self.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn num_constructor_extracts_number() {
        assert_eq!(ExpVal::num(42).as_num().unwrap(), 42);
    }

    #[test]
    fn boolean_constructor_extracts_boolean() {
        assert!(ExpVal::boolean(true).as_bool().unwrap());
    }

    #[test]
    fn wrong_extractor_reports_actual_value() {
        assert_eq!(
            ExpVal::boolean(false).as_num().unwrap_err(),
            ExpValError::NumValExpected {
                actual: ExpVal::boolean(false)
            }
        );
    }
}
