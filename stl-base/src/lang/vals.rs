use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, derive_more::Display)]
enum SimpleVal {
    #[display("{}", _0)]
    NumVal(i32),

    #[display("{}", _0)]
    BoolVal(bool),
}

pub enum ExpValError<V: ExpVal> {
    NumValExpected { actual: Rc<V> },
    BoolValExpected { actual: Rc<V> },
}

impl<V: ExpVal> Display for ExpValError<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpValError::NumValExpected { actual } => {
                write!(f, "expected number, found: {}", actual)
            }
            ExpValError::BoolValExpected { actual } => {
                write!(f, "expected boolean, found: {}", actual)
            }
        }
    }
}

impl<V: ExpVal> Debug for ExpValError<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<V: ExpVal> std::error::Error for ExpValError<V> {}

pub fn num_val(n: i32) -> Rc<impl ExpVal> {
    Rc::new(SimpleVal::NumVal(n))
}

pub fn bool_val(b: bool) -> Rc<impl ExpVal> {
    Rc::new(SimpleVal::BoolVal(b))
}

pub trait ExpVal: Debug + Display + Sized {
    fn to_num(&self) -> Result<i32, ExpValError<Self>>;
    fn to_bool(&self) -> Result<bool, ExpValError<Self>>;

    fn type_name(&self) -> &str;
}

impl ExpVal for SimpleVal {
    fn to_num(&self) -> Result<i32, ExpValError<Self>> {
        match self {
            SimpleVal::NumVal(n) => Ok(*n),
            SimpleVal::BoolVal(_) => Err(ExpValError::NumValExpected {
                actual: Rc::new(self.clone()),
            }),
        }
    }

    fn to_bool(&self) -> Result<bool, ExpValError<Self>> {
        match self {
            SimpleVal::NumVal(_) => Err(ExpValError::BoolValExpected {
                actual: Rc::new(self.clone()),
            }),
            SimpleVal::BoolVal(b) => Ok(*b),
        }
    }

    fn type_name(&self) -> &str {
        match self {
            SimpleVal::BoolVal(_) => "BoolVal",
            SimpleVal::NumVal(_) => "NumVal",
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_expval_numval() {
        let n42 = SimpleVal::NumVal(42);
        assert_eq!(n42.to_num().unwrap(), 42);
        let n33 = num_val(33);
        assert_eq!(
            n33.to_num().unwrap(),
            SimpleVal::NumVal(33).to_num().unwrap()
        );
    }

    #[test]
    fn test_expval_bool() {
        let btrue = SimpleVal::BoolVal(true);
        assert_eq!(btrue.to_bool().unwrap(), true);
        let bfalse = bool_val(false);
        assert_eq!(
            bfalse.to_bool().unwrap(),
            SimpleVal::BoolVal(false).to_bool().unwrap()
        );
    }
}
