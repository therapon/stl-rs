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
    VarExp(String),
    FnExp {
        params: Vec<String>,
        body: Box<Expr>,
    },
    CallExp {
        operator: Box<Expr>,
        operands: Vec<Expr>,
    },
    OrExp(Vec<Expr>),
    CondExp(Vec<CondClause>),
    LetExp {
        bindings: Vec<LetBinding>,
        body: Box<Expr>,
    },
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LetBinding {
    pub var: String,
    pub expr: Expr,
}

impl LetBinding {
    pub fn new(var: impl Into<String>, expr: Expr) -> Self {
        Self {
            var: var.into(),
            expr,
        }
    }
}
