use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpVal {
    NumVal(i64),
    BoolVal(bool),
}

impl Display for ExpVal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpVal::NumVal(n) => write!(f, "{n}"),
            ExpVal::BoolVal(b) => write!(f, "{b}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ExpValError {
    #[error("expected number, found {actual}")]
    NumValExpected { actual: ExpVal },

    #[error("expected boolean, found {actual}")]
    BoolValExpected { actual: ExpVal },
}

pub fn num_val(n: i64) -> ExpVal {
    ExpVal::NumVal(n)
}

pub fn bool_val(b: bool) -> ExpVal {
    ExpVal::BoolVal(b)
}

pub fn expval_to_num(val: &ExpVal) -> Result<i64, ExpValError> {
    match val {
        ExpVal::NumVal(n) => Ok(*n),
        ExpVal::BoolVal(_) => Err(ExpValError::NumValExpected {
            actual: val.clone(),
        }),
    }
}

pub fn expval_to_bool(val: &ExpVal) -> Result<bool, ExpValError> {
    match val {
        ExpVal::NumVal(_) => Err(ExpValError::BoolValExpected {
            actual: val.clone(),
        }),
        ExpVal::BoolVal(b) => Ok(*b),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn num_val_extracts_number() {
        assert_eq!(expval_to_num(&num_val(42)).unwrap(), 42);
    }

    #[test]
    fn bool_val_extracts_boolean() {
        assert!(expval_to_bool(&bool_val(true)).unwrap());
    }

    #[test]
    fn wrong_extractor_reports_actual_value() {
        assert_eq!(
            expval_to_num(&bool_val(false)).unwrap_err(),
            ExpValError::NumValExpected {
                actual: bool_val(false)
            }
        );
    }
}
