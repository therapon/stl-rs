#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub body: Expr,
}

impl Program {
    pub fn new(body: Expr) -> Self {
        Self { body }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    ConstExp(i64),
    BoolExp(bool),
    AddExp(Vec<Expr>),
    SubExp(Vec<Expr>),
    OrExp(Vec<Expr>),
    NotExp(Box<Expr>),
    CondExp(Vec<CondClause>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CondClause {
    pub condition: Expr,
    pub result: Expr,
}

impl CondClause {
    pub fn new(condition: Expr, result: Expr) -> Self {
        Self { condition, result }
    }
}
